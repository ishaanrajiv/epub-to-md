use anyhow::{Context, Result};
use clap::Parser;
use epub::doc::EpubDoc;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "epub-to-md")]
#[command(about = "Convert EPUB files to Markdown format", long_about = None)]
struct Cli {
    #[arg(help = "Path to the EPUB file")]
    input: PathBuf,

    #[arg(short, long, help = "Output directory for Markdown files")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Create a single merged Markdown file instead of separate files")]
    single: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate input file
    if !cli.input.exists() {
        anyhow::bail!("Input file does not exist: {}", cli.input.display());
    }

    if cli.input.extension().and_then(|s| s.to_str()) != Some("epub") {
        anyhow::bail!("Input file must have .epub extension");
    }

    // Determine output directory
    let output_dir = cli.output.unwrap_or_else(|| {
        let stem = cli.input.file_stem().unwrap();
        PathBuf::from(format!("{}_markdown", stem.to_string_lossy()))
    });

    // Convert EPUB to Markdown
    println!("Converting {} to Markdown...", cli.input.display());
    convert_epub_to_markdown(&cli.input, &output_dir, cli.single)?;

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

    println!("Title: {}", title);
    println!("Author: {}", author);
    println!("Processing {} resources...", doc.resources.len());

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

                println!("  Created: {}", filename);
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

        println!("  Created single file: {}", filepath.display());
    } else {
        println!("\nTotal chapters extracted: {}", chapter_num - 1);
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
