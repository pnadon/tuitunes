use clap::Parser;
use std::{
  error::Error,
  path::PathBuf,
  str::FromStr,
};

#[derive(Debug, Parser)]
struct Args {
  /// Path of the song, either local or a url.
  #[clap(short, long)]
  path: String,
  /// Change color based on the song
  #[clap(short, long)]
  color: bool,
}
fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  let path = if args.path.starts_with("https://") || args.path.starts_with("http://") {
    println!("Looks like you passed in a HTTP URL, downloading...");
    let path = tuitunes::songs::save_song_locally(&args.path)?;
    println!("Saved the file to disk, playing...");
    path
  } else {
    println!("Looks like you passed in a local path, playing...");
    PathBuf::from_str(&args.path)?
  };

  let res = tuitunes::app::run(path, args.color);
  if let Err(e) = res {
    println!("{:?}", e);
  }

  Ok(())
}
