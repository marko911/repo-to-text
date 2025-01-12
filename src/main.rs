use clap::Parser;
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Additional items to ignore (both directories and file extensions). Can be space or comma separated.
    #[arg(short, long, value_delimiter = ',', num_args = 1..)]
    ignore: Option<Vec<String>>,
}

struct RepoProcessor {
    output_file: String,
    ignored_dirs: HashSet<String>,
    ignored_exts: HashSet<String>,
    temp_dir: PathBuf,
}

impl RepoProcessor {
    fn new(additional_ignores: Option<Vec<String>>) -> io::Result<Self> {
        let temp_dir = tempfile::tempdir()?.into_path();

        let mut ignored_dirs: HashSet<String> =
            vec![".git", ".svn", "node_modules", "vendor", ".idea"]
                .into_iter()
                .map(String::from)
                .collect();

        // Match the fish script's ignored extensions exactly
        let mut ignored_exts: HashSet<String> = vec![
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
        ]
        .into_iter()
        .map(String::from)
        .collect();

        // Add user-provided extensions if any
        if let Some(additional) = additional_ignores {
            for item in additional {
                let clean_item = item.trim_start_matches('.');
                ignored_exts.insert(clean_item.to_string());
                ignored_dirs.insert(clean_item.to_string());
            }
        }

        Ok(Self {
            output_file: "repo_content.txt".to_string(),
            ignored_dirs,
            ignored_exts,
            temp_dir,
        })
    }

    fn should_ignore_dir(&self, dir: &str) -> bool {
        let dir_clean = dir.trim_start_matches('.');
        self.ignored_dirs.contains(dir) || self.ignored_dirs.contains(dir_clean)
    }

    fn should_ignore_ext(&self, file: &Path) -> bool {
        let filename = file
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        // Check if file has no extension
        if !filename.contains('.') || filename.ends_with('.') {
            return true;
        }

        // Check for .so.* pattern
        if filename.contains(".so.") {
            return true;
        }

        let extension = match file.extension() {
            Some(ext) => ext.to_string_lossy().to_string(),
            None => return true,
        };

        self.ignored_exts.contains(&extension)
    }

    fn collect_files(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    if let Some(dirname) = path.file_name() {
                        if !self.should_ignore_dir(&dirname.to_string_lossy()) {
                            files.extend(self.collect_files(&path)?);
                        }
                    }
                } else if path.is_file() {
                    if let Some(filename) = path.file_name() {
                        if !filename.to_string_lossy().starts_with("._")
                            && !self.should_ignore_ext(&path)
                        {
                            files.push(path);
                        }
                    }
                }
            }
        }

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

    pub fn process_repository(&self) -> io::Result<()> {
        let mut output = BufWriter::new(File::create(&self.output_file)?);

        writeln!(output, "Repository Content Extraction")?;
        writeln!(output, "Generated on: {:?}", SystemTime::now())?;
        writeln!(output, "=================================================")?;
        writeln!(output)?;

        println!("Collecting files...");
        let files = self.collect_files(Path::new("."))?;
        let total_files = files.len();

        println!("Processing {} files...", total_files);
        let processed_count = Arc::new(Mutex::new(0));
        let output_mutex = Arc::new(Mutex::new(BufWriter::new(File::create(&self.output_file)?)));

        // Process files in parallel using rayon's parallel iterator
        files.par_iter().try_for_each(|file| -> io::Result<()> {
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
    let processor = RepoProcessor::new(args.ignore)?;
    processor.process_repository()
}
