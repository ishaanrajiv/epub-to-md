# EPUB to Markdown Converter

A fast and efficient Rust CLI tool to convert EPUB files to Markdown format.

## Features

- Convert EPUB files to clean Markdown format
- Preserve chapter structure and book metadata
- Option to create separate files per chapter or a single combined file
- Fast and memory-efficient processing
- User-friendly error messages

## Installation

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/epub-to-md`

## Usage

### Basic Usage

Convert an EPUB file to multiple Markdown files (one per chapter):

```bash
./target/release/epub-to-md book.epub
```

This creates a directory named `book_markdown/` containing:
- `chapter_001.md`
- `chapter_002.md`
- `chapter_003.md`
- etc.

### Specify Output Directory

```bash
./target/release/epub-to-md book.epub -o output_folder
```

### Create Single Combined File

Convert to a single Markdown file instead of separate chapters:

```bash
./target/release/epub-to-md book.epub --single
```

This creates a single file with all chapters combined, separated by horizontal rules.

### Help

```bash
./target/release/epub-to-md --help
```

## Options

- `input` - Path to the EPUB file (required)
- `-o, --output <DIR>` - Output directory for Markdown files (default: `{book_name}_markdown`)
- `-s, --single` - Create a single merged Markdown file instead of separate files
- `-h, --help` - Print help information

## Examples

```bash
# Convert with custom output directory
./target/release/epub-to-md mybook.epub -o converted

# Create a single file
./target/release/epub-to-md mybook.epub --single

# Both options together
./target/release/epub-to-md mybook.epub -o converted --single
```

## How It Works

1. Parses the EPUB file structure
2. Extracts book metadata (title, author)
3. Iterates through chapters in reading order
4. Converts HTML content to clean Markdown
5. Outputs organized Markdown files

## Dependencies

- `clap` - Command-line argument parsing
- `epub` - EPUB file parsing
- `html2md` - HTML to Markdown conversion
- `anyhow` - Error handling

## License

MIT
