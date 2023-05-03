use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// Port to open worker on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    parsimon_worker::start(args.port)?;
    Ok(())
}
