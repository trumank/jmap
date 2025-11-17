use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "usmap")]
#[command(about = "Convert between usmap and JSON", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert usmap to JSON
    ToJson {
        /// Input usmap file
        input: PathBuf,

        /// Output JSON file (stdout if not specified)
        output: Option<PathBuf>,
    },
    /// Convert JSON to usmap
    FromJson {
        /// Input JSON file
        input: PathBuf,

        /// Output usmap file (stdout if not specified)
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ToJson { input, output } => {
            let input_file = std::fs::File::open(&input)
                .with_context(|| format!("Failed to open input file: {}", input.display()))?;
            let usmap = usmap::Usmap::read(&mut BufReader::new(input_file))
                .context("Failed to parse usmap")?;

            // Write JSON
            if let Some(output_path) = output {
                let output_file = std::fs::File::create(&output_path).with_context(|| {
                    format!("Failed to create output file: {}", output_path.display())
                })?;
                serde_json::to_writer_pretty(BufWriter::new(output_file), &usmap)
                    .context("Failed to write JSON")?;
            } else {
                serde_json::to_writer_pretty(std::io::stdout(), &usmap)
                    .context("Failed to write JSON")?;
            }
        }
        Commands::FromJson { input, output } => {
            let input_file = std::fs::File::open(&input)
                .with_context(|| format!("Failed to open input file: {}", input.display()))?;
            let usmap: usmap::Usmap = serde_json::from_reader(BufReader::new(input_file))
                .context("Failed to parse JSON")?;

            if let Some(output_path) = output {
                let output_file = std::fs::File::create(&output_path).with_context(|| {
                    format!("Failed to create output file: {}", output_path.display())
                })?;
                usmap
                    .write(&mut BufWriter::new(output_file))
                    .context("Failed to write usmap")?;
            } else {
                usmap
                    .write(&mut BufWriter::new(std::io::stdout()))
                    .context("Failed to write usmap")?;
            }
        }
    }

    Ok(())
}
