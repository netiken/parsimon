#[derive(Debug, clap::Parser)]
pub struct Session {
    #[clap(long, default_value = "./data")]
    root: PathBuf,
    #[clap(long)]
    mix: PathBuf,
    #[clap(long, default_value_t = 0)]
    seed: u64,
    #[clap(subcommand)]
    sim: SimKind,
}

impl Session {}
