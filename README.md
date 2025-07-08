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

### Including Extensions That Are Ignored by Default

If you need to _keep_ certain extensions that the tool normally skips (e.g. `json`, `yaml`) you can pass them with the `--include` flag.

```bash
# Process everything, but make sure .json and .yaml files are included
repo_to_text --include json,yaml

# Short form
repo_to_text -I json,yaml

# Combine with --ignore to fine-tune behaviour
repo_to_text --ignore log,tmp --include json
```

The list passed to `--include` takes precedence over both the built-in ignore list and any extensions provided through `--ignore`.

### Default Ignored Content

- **Directories**:

  - `.git`
  - `.svn`
  - `
