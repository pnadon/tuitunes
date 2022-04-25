use clap::Parser;
use std::{
  env::temp_dir,
  error::Error,
  fs::File,
  io::{Cursor},
  path::PathBuf,
  str::FromStr,
};
use tuitunes::song_vis::run;

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
    let resp = reqwest::blocking::get(args.path)?;
    let ext = resp
      .headers()
      .get("Content-Type")
      .map(|c| c.to_str())
      .unwrap_or(Ok("audio/mp3"))?
      .trim_start_matches("audio/");

    println!("Downloaded, looks like a {} file", ext);
    let path = {
      let mut d = temp_dir();
      d.push(format!("downloaded_song.{}", ext));
      d
    };

    let mut f = File::create(&path)?;
    let content = resp.bytes()?;
    std::io::copy(&mut Cursor::new(content), &mut f)?;
    println!("Saved the file to disk, playing...");
    path
  } else {
    println!("Looks like you passed in a local path, playing...");
    PathBuf::from_str(&args.path)?
  };
  let res = run(path, args.color);
  if let Err(e) = res {
    println!("{:?}", e);
  }

  Ok(())
}
