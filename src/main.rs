use clap::Parser;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use regex::Regex;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Additional items to ignore (both directories and file extensions). Can be space or comma separated.
    #[arg(short, long, value_delimiter = ',', num_args = 1..)]
    ignore: Option<Vec<String>>,
    /// File extensions to explicitly include (override default ignored extensions). Can be space or comma separated.
    #[arg(short = 'I', long, value_delimiter = ',', num_args = 1..)]
    include: Option<Vec<String>>,
}

struct RepoProcessor {
    output_file: String,
    ignored_dirs: HashSet<String>,
    ignored_exts: HashSet<String>,
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

        let mut ignored_dirs: HashSet<String> =
            vec!["git", "svn", "node_modules", "vendor", "idea", "target"]
                .into_iter()
                .map(String::from)
                .collect();

        // Match the fish script's ignored extensions exactly
        let mut ignored_exts: HashSet<String> = vec![
            "lock",
            "pack",
            "xz",
            "7z",
            "bz2",
            "gz",
            "lz",
            "lzma",
            "lzo",
            "rar",
            "tar",
            "xz",
            "z",
            "zip",
            "deb",
            "rpm",
            "apk",
            "ipa",
            "app",
            "dmg",
            "pkg",
            "exe",
            "dll",
            "csv",
            "so",
            "o",
            "a",
            "pyc",
            "class",
            "jar",
            "war",
            "woff",
            "woff2",
            "ttf",
            "eot",
            "env",
            "log",
            "gitignore",
            "json",
            "npmrc",
            "prettierrc",
            "eslintrc",
            "babelrc",
            "pyc",
            "pyo",
            "pyd",
            "class",
            "yml",
            "yaml",
            "jpg",
            "jpeg",
            "png",
            "gif",
            "bmp",
            "ico",
            "svg",
            "webp",
            "pdf",
            "bcmap", // Binary CMap files for PDF/font processing
            "pfb",   // Printer Font Binary
            "pfm",   // Printer Font Metrics
            "afm",   // Adobe Font Metrics
            "otf",   // OpenType Font
            "cff",   // Compact Font Format
            "fon",   // Legacy Windows Font format
        ]
        .into_iter()
        .map(String::from)
        .collect();

        // Add user-provided extensions to ignore, if any
        if let Some(additional) = additional_ignores {
            for item in additional {
                let clean_item = item.trim_start_matches('.');
                ignored_exts.insert(clean_item.to_string());
                ignored_dirs.insert(clean_item.to_string());
            }
        }

        // Remove explicitly included extensions from the ignored set
        if let Some(includes) = include_exts {
            for item in includes {
                let clean_item = item.trim_start_matches('.');
                let clean_item_lower = clean_item.to_lowercase();
                ignored_exts.remove(clean_item);
                ignored_exts.remove(&clean_item_lower);
                ignored_dirs.remove(clean_item);
                ignored_dirs.remove(&clean_item_lower);
            }
        }

        Ok(Self {
            output_file: "repo_content.txt".to_string(),
            ignored_dirs,
            ignored_exts,
            temp_dir,
            large_files: Arc::new(Mutex::new(Vec::new())),
            size_threshold: 1024 * 1024, // 1MB in bytes
        })
    }

    fn should_ignore_dir(&self, dir: &str) -> bool {
        let dir_clean = dir.trim_start_matches('.');
        self.ignored_dirs.contains(dir) || self.ignored_dirs.contains(dir_clean)
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

        self.ignored_exts.contains(&extension)
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

        let theme = ColorfulTheme::default();
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
    let processor = RepoProcessor::new(args.ignore, args.include)?;
    processor.process_repository()
}
