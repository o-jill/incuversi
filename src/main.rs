use std::io::Write;
use clap::Parser;

mod kifu;
mod bitboard;
// mod weight;
mod argument;
mod data_loader;
mod incubator;
mod ruversirunner;

fn main() -> Result<(), std::io::Error> {
    let arg = argument::Arg::parse();

    let mut inc = incubator::Incubator::from(arg);

    inc.run()
}
