use clap::Parser;
use std::{error::Error, path::PathBuf};
use tuitunes::song_vis::run;

#[derive(Debug, Parser)]
struct Args {
  /// Path of the song
  #[clap(short, long)]
  path: PathBuf,
  /// Change color based on the song
  #[clap(short, long)]
  color: bool,
}
fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();
  run(args.path, args.color)
}
