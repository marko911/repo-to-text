use clap::Parser;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use walkdir::WalkDir;

const DEFAULT_ALLOWED_EXTS: &[&str] = &[
    "ada",
    "adb",
    "ads",
    "apex",
    "as",
    "asm",
    "astro",
    "bas",
    "bat",
    "c",
    "cc",
    "cbl",
    "cl",
    "clj",
    "cljc",
    "cljs",
    "cls",
    "cmake",
    "coffee",
    "cr",
    "cs",
    "cshtml",
    "csh",
    "css",
    "cue",
    "cxx",
    "d",
    "dart",
    "edn",
    "elm",
    "erl",
    "ex",
    "exs",
    "fs",
    "fsi",
    "fsx",
    "fsscript",
    "f",
    "f03",
    "f08",
    "f77",
    "f90",
    "f95",
    "gd",
    "gemspec",
    "gleam",
    "glsl",
    "go",
    "gradle",
    "graphql",
    "gql",
    "groovy",
    "gvy",
    "h",
    "handlebars",
    "hbs",
    "hh",
    "hpp",
    "hs",
    "hx",
    "hxx",
    "htm",
    "html",
    "hrl",
    "hcl",
    "ipynb",
    "java",
    "jl",
    "js",
    "jsx",
    "json",
    "kql",
    "kt",
    "kts",
    "less",
    "liquid",
    "lua",
    "m",
    "mm",
    "ml",
    "mli",
    "mjs",
    "move",
    "nim",
    "nix",
    "odin",
    "php",
    "phtml",
    "pl",
    "pm",
    "proto",
    "ps1",
    "psm1",
    "pug",
    "purs",
    "py",
    "pyi",
    "pyx",
    "q",
    "r",
    "rb",
    "rego",
    "rs",
    "s",
    "sass",
    "scala",
    "sbt",
    "scm",
    "scss",
    "sh",
    "slim",
    "sol",
    "sql",
    "styl",
    "svelte",
    "swift",
    "tcl",
    "tf",
    "tfvars",
    "thrift",
    "ts",
    "tsx",
    "twig",
    "v",
    "vb",
    "vba",
    "vbs",
    "vh",
    "vue",
    "wgsl",
    "xml",
    "zig",
    "zsh",
];

const DEFAULT_IGNORED_DIRS: &[&str] = &[
    "__pycache__",
    "__snapshots__",
    "_build",
    "_output",
    "angular",
    "bazel-bin",
    "bazel-out",
    "bazel-testlogs",
    "bin",
    "bower_components",
    "build",
    "buck-out",
    "cache",
    "cmake-build-debug",
    "cmake-build-release",
    "coverage",
    "dart_tool",
    "debug",
    "deriveddata",
    "dist",
    "env",
    "git",
    "gradle",
    "hg",
    "jspm_packages",
    "logs",
    "m2",
    "mypy_cache",
    "next",
    "node_modules",
    "nuxt",
    "obj",
    "out",
    "output",
    "parcel-cache",
    "pnpm-store",
    "pods",
    "pytest_cache",
    "release",
    "reports",
    "ruff_cache",
    "serverless",
    "storybook-static",
    "svelte-kit",
    "svn",
    "target",
    "temp",
    "terraform",
    "tmp",
    "vendor",
    "venv",
    "vercel",
    "vs",
    "vscode",
    "yarn",
    "yarn_cache",
];

// Groq API structures
#[derive(Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct GroqRequest {
    messages: Vec<GroqMessage>,
    model: String,
    temperature: f32,
    max_completion_tokens: u32,
    top_p: f32,
    stream: bool,
    reasoning_effort: String,
}

#[derive(Deserialize, Debug)]
struct GroqStreamChoice {
    delta: GroqDelta,
}

#[derive(Deserialize, Debug)]
struct GroqDelta {
    content: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GroqStreamResponse {
    choices: Vec<GroqStreamChoice>,
}

/// Directories to skip during the initial scan (these are almost always noise)
const SCAN_SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "env",
    ".env",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "vendor",
    ".cargo",
    "deps",
    ".deps",
];

/// Scans a directory and collects all unique file extensions and directory names
/// Only scans top 3 levels and skips known dependency/build directories
fn collect_extensions_and_dirs(dir: &Path) -> (HashSet<String>, HashSet<String>) {
    let mut extensions: HashSet<String> = HashSet::new();
    let mut directories: HashSet<String> = HashSet::new();

    let skip_dirs: HashSet<&str> = SCAN_SKIP_DIRS.iter().copied().collect();

    for entry in WalkDir::new(dir)
        .max_depth(3) // Only scan top 3 levels
        .into_iter()
        .filter_entry(|e| {
            // Skip known problematic directories
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    let name_lower = name.to_lowercase();
                    return !skip_dirs.contains(name_lower.as_str());
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_dir() {
            if let Some(dirname) = path.file_name() {
                let name = dirname.to_string_lossy().to_lowercase();
                // Skip hidden directories from collection (but we still add them for AI to consider)
                if !name.is_empty() {
                    directories.insert(name);
                }
            }
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !ext_str.is_empty() {
                    extensions.insert(ext_str);
                }
            }
        }
    }

    (extensions, directories)
}

/// Calls the Groq LLM API to get suggestions for what to ignore
fn get_ai_ignore_suggestions(
    extensions: &HashSet<String>,
    directories: &HashSet<String>,
) -> io::Result<Vec<String>> {
    let api_key = match env::var("GROQ_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: GROQ_API_KEY not set, skipping AI-powered ignore suggestions");
            return Ok(vec![]);
        }
    };

    // Build the list of items to send to the LLM
    let mut items: Vec<String> = Vec::new();

    for ext in extensions {
        items.push(format!(".{}", ext));
    }
    for dir in directories {
        items.push(dir.clone());
    }

    if items.is_empty() {
        return Ok(vec![]);
    }

    let prompt = format!(
        "I am filtering a codebase with the following directories and file extensions for only files that are useful in understanding the function of the application. Which of these should I ignore? Send only a JSON array of strings back and nothing else.\n\n{}",
        items.join(", ")
    );

    println!("Asking AI for smart ignore suggestions...");

    let client = reqwest::blocking::Client::new();

    let request = GroqRequest {
        messages: vec![GroqMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        model: "qwen/qwen3-32b".to_string(),
        temperature: 0.6,
        max_completion_tokens: 4096,
        top_p: 0.95,
        stream: true,
        reasoning_effort: "default".to_string(),
    };

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("HTTP request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Groq API error ({}): {}", status, body),
        ));
    }

    // Process streaming response
    let mut full_content = String::new();

    for line in response.text().unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line == "data: [DONE]" {
            continue;
        }

        if let Some(json_str) = line.strip_prefix("data: ") {
            if let Ok(stream_response) = serde_json::from_str::<GroqStreamResponse>(json_str) {
                for choice in stream_response.choices {
                    if let Some(content) = choice.delta.content {
                        full_content.push_str(&content);
                    }
                }
            }
        }
    }

    // Parse the JSON array from the response
    // First, try to extract JSON array from the response (it might have extra text)
    let json_start = full_content.find('[');
    let json_end = full_content.rfind(']');

    let suggestions: Vec<String> = match (json_start, json_end) {
        (Some(start), Some(end)) if end > start => {
            let json_str = &full_content[start..=end];
            serde_json::from_str(json_str).unwrap_or_else(|e| {
                eprintln!("Warning: Failed to parse AI response as JSON: {}", e);
                eprintln!("Response was: {}", full_content);
                vec![]
            })
        }
        _ => {
            eprintln!("Warning: Could not find JSON array in AI response");
            eprintln!("Response was: {}", full_content);
            vec![]
        }
    };

    if !suggestions.is_empty() {
        println!("AI suggests ignoring: {:?}", suggestions);
    }

    Ok(suggestions)
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Extra directories or extensions to ignore even if they would normally be included. Can be space or comma separated.
    #[arg(short, long, value_delimiter = ',', num_args = 1..)]
    ignore: Option<Vec<String>>,
    /// Additional file extensions to include beyond the default list of programming languages. Can be space or comma separated.
    #[arg(short = 'I', long, value_delimiter = ',', num_args = 1..)]
    include: Option<Vec<String>>,
    /// Disable AI-powered ignore suggestions (requires GROQ_API_KEY env var when enabled)
    #[arg(long)]
    no_ai: bool,
}

struct RepoProcessor {
    output_file: String,
    ignored_dirs: HashSet<String>,
    allowed_exts: HashSet<String>,
    temp_dir: PathBuf,
    large_files: Arc<Mutex<Vec<(PathBuf, u64)>>>,
    size_threshold: u64,
}

impl RepoProcessor {
    fn new(
        additional_ignores: Option<Vec<String>>,
        include_exts: Option<Vec<String>>,
    ) -> io::Result<Self> {
        let temp_dir = tempfile::tempdir()?.into_path();

        let mut ignored_dirs: HashSet<String> = DEFAULT_IGNORED_DIRS
            .iter()
            .map(|d| d.trim_start_matches('.').to_lowercase())
            .collect();

        let mut allowed_exts: HashSet<String> = DEFAULT_ALLOWED_EXTS
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_lowercase())
            .collect();

        // Add user-provided extensions to ignore, if any
        if let Some(additional) = additional_ignores {
            for item in additional {
                let cleaned = item.trim();
                if cleaned.is_empty() {
                    continue;
                }

                let dir_name = cleaned.trim_start_matches('.').to_lowercase();
                if !dir_name.is_empty() {
                    ignored_dirs.insert(dir_name.clone());
                    allowed_exts.remove(&dir_name);
                }
            }
        }

        if let Some(includes) = include_exts {
            for item in includes {
                let cleaned = item.trim();
                if cleaned.is_empty() {
                    continue;
                }

                let ext = cleaned.trim_start_matches('.').to_lowercase();
                if !ext.is_empty() {
                    allowed_exts.insert(ext);
                }
            }
        }

        Ok(Self {
            output_file: "repo_content.txt".to_string(),
            ignored_dirs,
            allowed_exts,
            temp_dir,
            large_files: Arc::new(Mutex::new(Vec::new())),
            size_threshold: 1024 * 1024, // 1MB in bytes
        })
    }

    fn should_ignore_dir(&self, dir: &str) -> bool {
        let dir_lower = dir.to_lowercase();
        let dir_clean = dir_lower.trim_start_matches('.');
        self.ignored_dirs.contains(dir_clean)
    }

    fn should_ignore_ext(&self, file: &Path) -> bool {
        let filename = file
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();

        // Check if file has no extension
        if !filename.contains('.') || filename.ends_with('.') {
            return true;
        }

        // Check for .so.* pattern (case-insensitive)
        if filename.to_lowercase().contains(".so.") {
            return true;
        }

        let extension = match file.extension() {
            Some(ext) => ext.to_string_lossy().to_string().to_lowercase(),
            None => return true,
        };

        if extension.is_empty() {
            return true;
        }

        !self.allowed_exts.contains(&extension)
    }

    fn collect_files(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let files: Vec<PathBuf> = WalkDir::new(dir)
            .into_iter()
            // Skip entries whose parent directories are in the ignored list
            .filter_entry(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dirname) = path.file_name() {
                        return !self.should_ignore_dir(&dirname.to_string_lossy());
                    }
                }
                true
            })
            // Convert the iterator to a parallel iterator
            .par_bridge()
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let path = e.into_path();

                    if !path.is_file() {
                        return None;
                    }

                    // Skip unwanted files
                    if let Some(filename) = path.file_name() {
                        if filename.to_string_lossy().starts_with("._")
                            || self.should_ignore_ext(&path)
                        {
                            return None;
                        }
                    }

                    // Track large files
                    if let Ok(metadata) = path.metadata() {
                        let size = metadata.len();
                        if size > self.size_threshold {
                            self.large_files.lock().unwrap().push((path.clone(), size));
                        }
                    }

                    Some(path)
                }
                Err(_) => None,
            })
            .collect();

        Ok(files)
    }

    fn process_file(&self, file: &Path) -> io::Result<PathBuf> {
        let hash = format!("{:x}", md5::compute(file.to_string_lossy().as_bytes()));
        let outfile = self.temp_dir.join(format!("{}.txt", hash));

        let mut writer = BufWriter::new(File::create(&outfile)?);

        writeln!(writer, "===============================================")?;
        writeln!(writer, "--- File: {} ---", file.display())?;
        writeln!(writer, "===============================================")?;
        writeln!(writer)?;

        // Read file as bytes instead of UTF-8 string
        let content = fs::read(file)?;

        // Convert to string, replacing invalid UTF-8 with placeholder
        let content = String::from_utf8_lossy(&content);

        // Process content with the same patterns as the fish script
        let binary_patterns = [
            (
                r#"(?s)(DATA = b""")[^"]*?(""")"#,
                r#"$1<binary data removed>$2"#,
            ),
            (
                r#"(?s)(b85decode\().*?(\))"#,
                r#"$1"<binary data removed>"$2"#,
            ),
            (
                r#"(?s)(base64\.[^(]*decode\().*?(\))"#,
                r#"$1"<binary data removed>"$2"#,
            ),
        ];

        let processed_content =
            binary_patterns
                .iter()
                .fold(content.to_string(), |acc, (pattern, replacement)| {
                    Regex::new(pattern)
                        .unwrap()
                        .replace_all(&acc, *replacement)
                        .to_string()
                });

        write!(writer, "{}", processed_content)?;
        writeln!(writer)?;
        writeln!(writer, "--- End of File ---")?;
        writeln!(writer)?;
        writeln!(writer, "===============================================")?;

        Ok(outfile)
    }

    fn prompt_large_files(&self, files: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
        if self.large_files.lock().unwrap().is_empty() {
            return Ok(files.to_vec());
        }

        println!("\nFound large files (>1MB). Use ↑↓ to navigate, Y/N to select, Enter when done:");
        let items: Vec<String> = self
            .large_files
            .lock()
            .unwrap()
            .iter()
            .map(|(path, size)| {
                format!(
                    "{} ({:.2}MB)",
                    path.display(),
                    *size as f64 / (1024.0 * 1024.0)
                )
            })
            .collect();

        let mut current_selection = vec![true; items.len()];
        let mut current_index = 0;

        loop {
            // Clear screen and show current state
            print!("\x1B[2J\x1B[1;1H");
            println!("Select files to include (Y/N for current item, ↑↓ to navigate, Enter to finish):\n");

            for (idx, item) in items.iter().enumerate() {
                let prefix = if idx == current_index { ">" } else { " " };
                let status = if current_selection[idx] { "Y" } else { "N" };
                println!("{} [{}] {}", prefix, status, item);
            }

            // Get user input
            if let Ok(key) = dialoguer::console::Term::stdout().read_key() {
                match key {
                    dialoguer::console::Key::Char('y') | dialoguer::console::Key::Char('Y') => {
                        current_selection[current_index] = true;
                        if current_index < items.len() - 1 {
                            current_index += 1;
                        }
                    }
                    dialoguer::console::Key::Char('n') | dialoguer::console::Key::Char('N') => {
                        current_selection[current_index] = false;
                        if current_index < items.len() - 1 {
                            current_index += 1;
                        }
                    }
                    dialoguer::console::Key::ArrowUp if current_index > 0 => {
                        current_index -= 1;
                    }
                    dialoguer::console::Key::ArrowDown if current_index < items.len() - 1 => {
                        current_index += 1;
                    }
                    dialoguer::console::Key::Enter => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let selected_paths: HashSet<_> = self
            .large_files
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(i, _)| current_selection[*i])
            .map(|(_, (path, _))| path.clone())
            .collect();

        Ok(files
            .iter()
            .filter(|f| {
                if self
                    .large_files
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|(p, _)| p == *f)
                {
                    selected_paths.contains(*f)
                } else {
                    true
                }
            })
            .cloned()
            .collect())
    }

    pub fn process_repository(&self) -> io::Result<()> {
        let mut output = BufWriter::new(File::create(&self.output_file)?);

        writeln!(output, "Repository Content Extraction")?;
        writeln!(output, "Generated on: {:?}", SystemTime::now())?;
        writeln!(output, "=================================================")?;
        writeln!(output)?;

        println!("Collecting files...");
        let files = self.collect_files(Path::new("."))?;

        // Prompt for large files before processing
        let files_to_process = self.prompt_large_files(&files)?;
        let total_files = files_to_process.len();

        println!("Processing {} files...", total_files);
        let processed_count = Arc::new(Mutex::new(0));
        let output_mutex = Arc::new(Mutex::new(BufWriter::new(File::create(&self.output_file)?)));

        // Process files in parallel using rayon's parallel iterator
        files_to_process
            .par_iter()
            .try_for_each(|file| -> io::Result<()> {
                let count = {
                    let mut count = processed_count.lock().unwrap();
                    *count += 1;
                    *count
                };

                print!(
                    "\rProcessing file {} of {}: {}",
                    count,
                    total_files,
                    file.display()
                );
                io::stdout().flush()?;

                let temp_file = self.process_file(file)?;
                let content = fs::read_to_string(&temp_file)?;

                // Write directly to the output file under lock
                let mut output = output_mutex.lock().unwrap();
                write!(output, "{}", content)?;

                // Clean up temp file immediately
                fs::remove_file(temp_file)?;

                Ok(())
            })?;

        println!(
            "\nFinished processing. Output saved to {}",
            self.output_file
        );

        // Cleanup temp directory
        fs::remove_dir_all(&self.temp_dir)?;

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Determine ignore patterns
    let ignore_patterns = if args.ignore.is_some() {
        // User provided explicit ignores, use those
        args.ignore
    } else if !args.no_ai {
        // No explicit ignores and AI is enabled, get suggestions
        let target_dir = Path::new(".");
        println!("Scanning directory for extensions and folders...");
        let (extensions, directories) = collect_extensions_and_dirs(target_dir);

        println!(
            "Found {} unique extensions and {} directories",
            extensions.len(),
            directories.len()
        );

        match get_ai_ignore_suggestions(&extensions, &directories) {
            Ok(suggestions) if !suggestions.is_empty() => Some(suggestions),
            Ok(_) => None,
            Err(e) => {
                eprintln!("Warning: AI suggestion failed: {}", e);
                None
            }
        }
    } else {
        // AI is disabled and no explicit ignores
        None
    };

    let processor = RepoProcessor::new(ignore_patterns, args.include)?;
    processor.process_repository()
}
