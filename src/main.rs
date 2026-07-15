use std::io::Write;
use clap::Parser;

mod argument;
mod bitboard;
mod cassio;
mod cassiorunner;
mod data_loader;
mod incubator;
mod kifu;
mod ruversirunner;

fn main() -> Result<(), std::io::Error> {
    let arg = argument::Arg::parse();

    let mut inc = incubator::Incubator::from(arg);

    inc.run()
}