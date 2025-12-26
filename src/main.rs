use anyhow::{Context, Result};
use clap::Parser;
use epub::doc::EpubDoc;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "epub-to-md")]
#[command(about = "Convert EPUB files to Markdown format", long_about = None)]
struct Cli {
    #[arg(help = "Path to an EPUB file or a directory containing EPUB files")]
    input: PathBuf,

    #[arg(short, long, help = "Output directory for Markdown files")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Create a single merged Markdown file instead of separate files")]
    single: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate input exists
    if !cli.input.exists() {
        anyhow::bail!("Input path does not exist: {}", cli.input.display());
    }

    // Check if input is a directory or a file
    if cli.input.is_dir() {
        process_directory(&cli.input, cli.output.as_deref(), cli.single)?;
    } else {
        // Single file processing
        if cli.input.extension().and_then(|s| s.to_str()) != Some("epub") {
            anyhow::bail!("Input file must have .epub extension");
        }
        process_single_epub(&cli.input, cli.output.as_deref(), cli.single)?;
    }

    Ok(())
}

/// Recursively find all EPUB files in a directory
fn find_epub_files(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().and_then(|s| s.to_str()) == Some("epub")
                && entry.file_type().is_file()
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

/// Process all EPUB files in a directory in parallel
fn process_directory(dir: &Path, output_base: Option<&Path>, single_file: bool) -> Result<()> {
    let epub_files = find_epub_files(dir);

    if epub_files.is_empty() {
        anyhow::bail!("No EPUB files found in directory: {}", dir.display());
    }

    println!("Found {} EPUB file(s) in {}", epub_files.len(), dir.display());
    println!("Processing in parallel...\n");

    // Process all EPUB files in parallel
    let results: Vec<Result<(), anyhow::Error>> = epub_files
        .par_iter()
        .map(|epub_path| {
            let output_dir = if let Some(base) = output_base {
                // Create output path that mirrors the input directory structure
                let relative = epub_path.strip_prefix(dir).unwrap_or(epub_path);
                let stem = relative.file_stem().unwrap_or_default();
                base.join(format!("{}_markdown", stem.to_string_lossy()))
            } else {
                // Default: create output next to the epub file
                let parent = epub_path.parent().unwrap_or_else(|| Path::new("."));
                let stem = epub_path.file_stem().unwrap_or_default();
                parent.join(format!("{}_markdown", stem.to_string_lossy()))
            };

            convert_epub_to_markdown(epub_path, &output_dir, single_file)
        })
        .collect();

    // Report results
    let mut success_count = 0;
    let mut error_count = 0;

    for (path, result) in epub_files.iter().zip(results.iter()) {
        match result {
            Ok(()) => success_count += 1,
            Err(e) => {
                error_count += 1;
                eprintln!("Failed to process {}: {}", path.display(), e);
            }
        }
    }

    println!("\n--- Summary ---");
    println!("Successfully processed: {}", success_count);
    if error_count > 0 {
        println!("Failed: {}", error_count);
        anyhow::bail!("{} EPUB file(s) failed to process", error_count);
    }

    Ok(())
}

/// Process a single EPUB file
fn process_single_epub(epub_path: &Path, output_base: Option<&Path>, single_file: bool) -> Result<()> {
    let output_dir = if let Some(base) = output_base {
        base.to_path_buf()
    } else {
        let stem = epub_path.file_stem().unwrap();
        PathBuf::from(format!("{}_markdown", stem.to_string_lossy()))
    };

    println!("Converting {} to Markdown...", epub_path.display());
    convert_epub_to_markdown(epub_path, &output_dir, single_file)?;
    println!("Conversion complete! Output saved to: {}", output_dir.display());

    Ok(())
}

fn convert_epub_to_markdown(epub_path: &Path, output_dir: &Path, single_file: bool) -> Result<()> {
    // Open the EPUB document
    let mut doc = EpubDoc::new(epub_path)
        .context("Failed to open EPUB file")?;

    // Create output directory if it doesn't exist
    if !single_file {
        fs::create_dir_all(output_dir)
            .context("Failed to create output directory")?;
    }

    // Get book metadata
    let title = doc.mdata("title")
        .map(|m| m.value.clone())
        .unwrap_or_else(|| "Unknown Title".to_string());
    let author = doc.mdata("creator")
        .map(|m| m.value.clone())
        .unwrap_or_else(|| "Unknown Author".to_string());

    println!("  [{}] Title: {}, Author: {}", 
        epub_path.file_name().unwrap_or_default().to_string_lossy(),
        title, 
        author
    );

    let mut all_content = String::new();

    // Add metadata to combined file
    if single_file {
        all_content.push_str(&format!("# {}\n\n", title));
        all_content.push_str(&format!("**Author:** {}\n\n", author));
        all_content.push_str("---\n\n");
    }

    // Iterate through spine (reading order)
    let mut chapter_num = 1;
    let spine_len = doc.spine.len();

    for i in 0..spine_len {
        doc.set_current_chapter(i);

        if let Some((content, _mime)) = doc.get_current_str() {
            // Convert HTML to Markdown
            let markdown = html2md::parse_html(&content);

            // Skip empty or minimal content
            if markdown.trim().is_empty() || markdown.trim().len() < 50 {
                continue;
            }

            if single_file {
                // Append to combined content
                all_content.push_str(&markdown);
                all_content.push_str("\n\n---\n\n");
            } else {
                // Save as separate file
                let filename = format!("chapter_{:03}.md", chapter_num);
                let filepath = output_dir.join(&filename);

                fs::write(&filepath, &markdown)
                    .context(format!("Failed to write {}", filename))?;
            }

            chapter_num += 1;
        }
    }

    // Write single combined file if requested
    if single_file {
        let filename = format!("{}.md", sanitize_filename(&title));
        let filepath = output_dir.parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&filename);

        fs::write(&filepath, all_content)
            .context("Failed to write combined Markdown file")?;
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
