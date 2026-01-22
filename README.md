# repo_to_text

A fast, efficient command-line tool that converts an entire code repository into a single text file, making it easy to feed your codebase into LLMs (Large Language Models).

Asking questions of your codebase to big LLM providers is surprisingly tricky because of file number limits and size limits. This tool is to help you get around that.

## Features

- 🚀 Fast parallel processing using Rayon
- 🧹 Intelligent filtering of binary and non-source files
- 🔍 Smart handling of binary data blocks in source files
- 📝 Clean, formatted output with file separators
- 🛠️ Configurable file extension filtering
- 🤖 **AI-powered smart ignore suggestions** (via Groq API)

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

### From GitHub Releases

Download the pre-built binary for your platform from the [releases page](https://github.com/marko911/repo_to_text/releases).

## Usage

Basic usage (run from within your repository):

```bash
repo_to_text
```

The tool will create a `repo_content.txt` file in the current directory containing all your repository's text content.

## AI-Powered Smart Ignore (New!)

When you run `repo_to_text` without the `-i` flag, it will automatically:

1. Scan your directory for all unique file extensions and folder names
2. Ask the Groq LLM API which ones should be ignored
3. Apply those suggestions as the ignore list

### Setup

Set your Groq API key as an environment variable:

```bash
export GROQ_API_KEY="your-api-key-here"
```

You can get a free API key at [console.groq.com](https://console.groq.com).

### Example

```bash
# Let AI decide what to ignore
repo_to_text

# Output:
# Scanning directory for extensions and folders...
# Found 16 unique extensions and 26 directories
# Asking AI for smart ignore suggestions...
# AI suggests ignoring: [".lock", ".log", "dist", "coverage", ...]
# Collecting files...
```

### Disable AI Suggestions

If you don't want AI suggestions (or don't have an API key):

```bash
# Disable AI completely
repo_to_text --no-ai

# Or provide your own ignore list (AI is skipped when -i is provided)
repo_to_text -i node_modules,dist,.git
```

## Manual Ignore/Include

### Ignoring Additional File Extensions

You can specify additional file extensions or directories to ignore:

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

## CLI Reference

```
Usage: repo_to_text [OPTIONS]

Options:
  -i, --ignore <IGNORE>    Extra directories or extensions to ignore (comma-separated)
  -I, --include <INCLUDE>  Additional file extensions to include (comma-separated)
      --no-ai              Disable AI-powered ignore suggestions
  -h, --help               Print help
  -V, --version            Print version
```

## Default Ignored Content

### Directories

The following directories are ignored by default:

- Version control: `.git`, `.svn`, `.hg`
- Dependencies: `node_modules`, `vendor`, `bower_components`
- Build outputs: `dist`, `build`, `out`, `target`, `bin`, `obj`
- Caches: `__pycache__`, `.pytest_cache`, `.mypy_cache`, `cache`
- Virtual environments: `venv`, `.venv`, `env`
- IDE/Editor: `.vscode`, `.idea`
- And many more...

### File Extensions

Only source code files are included by default (100+ programming language extensions). Binary files, images, and other non-text files are automatically excluded.

## Large File Handling

When the tool encounters files larger than 1MB, it will prompt you to select which ones to include:

```
Found large files (>1MB). Use ↑↓ to navigate, Y/N to select, Enter when done:
> [Y] ./data/large_dataset.json (2.34MB)
  [N] ./assets/bundle.js (1.56MB)
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GROQ_API_KEY` | API key for Groq LLM (enables AI-powered ignore suggestions) |

## License

MIT
