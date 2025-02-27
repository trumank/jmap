use anyhow::{bail, Result};
use clap::Parser;
use dumper::{Input, StructInfo};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Dump from process ID
    #[arg(long, short, group = "input")]
    process: Option<i32>,

    /// Dump from minidump
    #[arg(long, short, group = "input")]
    dump: Option<PathBuf>,

    /// Struct layout info .json (from pdb2json)
    #[arg(index = 1)]
    struct_info: PathBuf,

    /// Output dump .json path
    #[arg(index = 2)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = match (cli.process, cli.dump) {
        (Some(number), None) => Input::Process(number),
        (None, Some(path)) => Input::Dump(path),
        (None, None) => {
            bail!("Error: Requires --process or --dump");
        }
        (Some(_), Some(_)) => {
            bail!("Error: Must specify either --process OR --dump");
        }
    };

    let struct_info: Vec<StructInfo> = serde_json::from_slice(&std::fs::read(cli.struct_info)?)?;

    let objects = dumper::dump(input, struct_info)?;

    std::fs::write(cli.output, serde_json::to_vec(&objects)?)?;

    Ok(())
}
