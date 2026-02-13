use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, author, about)]
pub struct Arg {
    /// mode
    #[command(subcommand)]
    pub md : Mode,
    /// kifu directory
    #[arg(long, global = true, value_delimiter=',')]
    pub kifudir : Vec<String>,
    /// show progressbar
    #[arg(long, global = true, default_value_t = false)]
    pub progressbar : bool,
    /// log file path.
    #[arg(long, global = true)]
    pub log : Option<String>,
    /// get mate(N-1) positions by extracting mateN.
    #[arg(long, short, global = true, default_value_t = 3)]
    pub  mate : u32,
    /// ruversi config file
    #[arg(long, global = true)]
    pub ru_config : Option<String>,
    /// show details
    #[arg(long, global = true, default_value_t=false)]
    pub verbose : bool,
}

#[derive(Debug, Subcommand)]
pub enum Mode {
    Kifu,  // Extract from kifu files
    Mate,  // Extract from mate files
}
