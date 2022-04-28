use clap::Parser;
use std::{error::Error, path::PathBuf, str::FromStr};

#[derive(Debug, Parser)]
struct Args {
  /// Path of the song, either local or a url.
  #[clap(short, long)]
  path: Option<String>,
  /// Use the default color for the ui, instead of changing per-song.
  #[clap(short, long)]
  default_color: bool,
}

/// Handles parsing arguments, and then passing them to the app.
fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();


  let path = args.path.map(|p| {
    if p.starts_with("https://") || p.starts_with("http://") {
      println!("Looks like you passed in a HTTP URL, downloading...");
      let path = tuitunes::songs::save_song_locally(&p).unwrap();
      println!("Saved the file to disk, playing...");
      path
    } else {
      println!("Looks like you passed in a local path, playing...");
      PathBuf::from_str(&p).unwrap()
    }
  });

  let res = tuitunes::app::run(path, args.default_color);
  if let Err(e) = res {
    println!("{:?}", e);
  }

  Ok(())
}
