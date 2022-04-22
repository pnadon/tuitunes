use clap::Parser;
use std::error::Error;
use tuitunes::song_vis::run;

#[derive(Debug, Parser)]
struct Args {
  /// Path of the song
  #[clap(short, long)]
  path: String,
  /// Disable brightness adjustment based on value, currently unused
  #[clap(short, long)]
  no_brightness: bool,
}
fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();
  run(&args.path)
}
