use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, author, about)]
pub struct Arg {
    /// kifu directory
    #[arg(long)]
    pub kifudir : Option<String>,
    /// show progressbar
    #[arg(long, default_value_t = false)]
    pub progressbar : bool,
    /// log file path.
    #[arg(long)]
    pub log : Option<String>,
    /// get mate(N-1) positions by extracting mateN.
    #[arg(long, short, default_value_t = 3)]
    pub  mate : u32,
    /// ruversi config file
    #[arg(long)]
    pub ru_config : Option<String>,
}
