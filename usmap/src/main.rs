use anyhow::Result;

fn main() -> Result<()> {
    let usmap = usmap::Usmap::read(&mut std::io::BufReader::new(std::fs::File::open(
        std::env::args().nth(1).unwrap(),
    )?))?;
    serde_json::to_writer(std::io::stdout(), &usmap)?;
    Ok(())
}
