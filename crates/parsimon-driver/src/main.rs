use clap::Parser;

fn main() -> anyhow::Result<()> {
    let expt = Session::parse();
    expt.run()?;
    Ok(())
}
