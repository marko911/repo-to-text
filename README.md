# repo_to_text

A fast, efficient command-line tool that converts an entire code repository into a single text file, making it easy to feed your codebase into LLMs (Large Language Models).
Asking questions of your codebase to big LLM providers is surprisingly tricky because of file number limits and size limits. This tool is to help you get around that.

## Features

- 🚀 Fast parallel processing using Rayon
- 🧹 Intelligent filtering of binary and non-source files
- 🔍 Smart handling of binary data blocks in source files
- 📝 Clean, formatted output with file separators
- 🛠️ Configurable file extension filtering

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/marko911/repo_to_text
cd repo_to_text

# Install to /usr/local/bin (requires sudo)
cargo install --path . --root /usr/local
```

### Manual Installation

```bash
# Build the release binary
cargo build --release

# Copy to /usr/local/bin (requires sudo)
sudo cp target/release/repo_to_text /usr/local/bin/
```

## Usage

Basic usage (run from within your repository):

```bash
repo_to_text
```

The tool will create a `repo_content.txt` file in the current directory containing all your repository's text content.

### Ignoring Additional File Extensions

You can specify additional file extensions to ignore:

```bash
repo_to_text --ignore txt,md,conf

# Or using the short form
repo_to_text -i txt,md,conf
```

### Default Ignored Content

- **Directories**:

  - `.git`
  - `.svn`
  - `node_modules`
  - `vendor`
  - `.idea`
  - `target`
  - `dist`
  - `build`
  - `.next`
  - `coverage`
  - `__pycache__`
  - `.pytest_cache`

- **File Extensions**:

  - Binary files: `exe`, `dll`, `so`, `dylib`, `bin`
  - Archives: `zip`, `tar`, `gz`, `rar`, `7z`
  - Images: `jpg`, `jpeg`, `png`, `gif`, `bmp`, `ico`, `svg`
  - Audio/Video: `mp3`, `mp4`, `wav`, `avi`, `mov`
  - Documents: `pdf`, `doc`, `docx`
  - Database: `db`, `sqlite`, `sqlite3`
  - Compiled: `pyc`, `class`, `o`
  - Package files: `lock`, `sum`

- **Special Files**:
  - macOS system files (starting with `._`)
  - Files without extensions
  - `.DS_Store`
  - `.env`
  - `.log`

## Output Format

The generated `repo_content.txt` file follows this format:

```
Repository Content Extraction
Generated on: 2024-12-22 08:13:01

===============================================
--- File: src/main.rs ---
===============================================
[file content here]
--- End of File ---
===============================================
```

## Use Cases

- Training custom LLMs on your codebase
- Creating context for LLM prompts
- Code analysis and documentation

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
