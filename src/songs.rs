use std::{
  env::{self, temp_dir},
  error::Error,
  fs::File,
  io::{BufReader, Cursor, Read},
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

use rodio::{Decoder, OutputStreamHandle, Sink, Source};

use anyhow::anyhow;

use crate::spectrum::Analyzer;

/// Checks if the `song` path is a supported format, and loads it.
pub fn load_app_and_sink<'a>(
  song: &'a PathBuf,
  stream_handle: &OutputStreamHandle,
) -> Result<(Analyzer<'a>, Sink), Box<dyn Error>> {
  if !has_supported_extension(song) {
    return Err(anyhow!("file {} is not a supported format", song.to_str().unwrap()).into());
  }
  let sink = stream_handle.play_once(BufReader::new(File::open(song)?))?;

  let file = BufReader::new(File::open(song)?);

  let app = crate::spectrum::Analyzer::new(Decoder::new(file)?.convert_samples::<f32>());

  Ok((app, sink))
}

/// Helper function to determine is a file is a supported format.
fn has_supported_extension(path: &Path) -> bool {
  crate::SUPPORTED_FORMATS
    .iter()
    .any(|ext| path.extension().and_then(|e| e.to_str()) == Some(*ext))
}

/// Takes a list of song paths, and returns a list with just the names of the files.
pub fn to_song_names(paths: &[PathBuf], rev: bool) -> Vec<&str> {
  let p = paths
    .iter()
    .map(|b| b.file_stem().unwrap().to_str().unwrap());
  if rev {
    p.rev().take(20).collect::<Vec<&str>>()
  } else {
    p.take(20).collect::<Vec<&str>>()
  }
}

/// Checks the path, if it's a directory it loads all of the songs in it.
/// Otherwise if its a file it will attempt to load it as a song.
pub fn load_song_list(song_path: PathBuf) -> std::io::Result<Vec<PathBuf>> {
  let mut s = if song_path.is_dir() {
    song_path
      .read_dir()?
      .filter_map(|e| e.ok())
      .filter(|e| e.metadata().unwrap().is_file() && has_supported_extension(&e.path()))
      .map(|e| e.path())
      .collect::<Vec<PathBuf>>()
  } else {
    vec![song_path]
  };
  s.sort();
  s.reverse();
  Ok(s)
}

/// Performs an HTTP request and saves the file to a temporary location.
pub fn save_song_locally(path: &str) -> Result<PathBuf, Box<dyn Error>> {
  let resp = reqwest::blocking::get(path)?;
  let ext = resp
    .headers()
    .get("Content-Type")
    .map(|c| c.to_str())
    .unwrap_or(Ok("audio/mp3"))?
    .trim_start_matches("audio/");

  let path = {
    let mut d = temp_dir();
    d.push(format!("downloaded_song.{}", ext));
    d
  };

  let mut f = File::create(&path)?;
  let content = resp.bytes()?;
  std::io::copy(&mut Cursor::new(content), &mut f)?;
  Ok(path)
}

/// Returns a path for the purpose of pre-populating
/// the search.
/// It will first check if `MUSIC_HOME` is set,
/// then `HOME`, and then by default return an empty list.
pub fn get_search_dir() -> String {
  if let Ok(s) = env::var("MUSIC_HOME") {
    return s;
  }
  if let Ok(s) = env::var("HOME") {
    return s;
  }
  "".to_owned()
}

pub fn search_songs(query: &str, buf: &mut String) -> Result<(), Box<dyn Error>> {
  let path = PathBuf::from(query);
  let (name, dir) = if path.is_dir() {
    (".", path.to_str().unwrap_or("/"))
  } else {
    (
      path.file_name().and_then(|s| s.to_str()).unwrap_or("."),
      path.parent().and_then(|s| s.to_str()).unwrap_or("/"),
    )
  };

  let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
  let mut command = Command::new(shell)
    .arg("-c")
    .arg(format!("fd \"{}\" \"{}\" -d 1", name, dir))
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  command
    .stdout
    .take()
    .ok_or_else(|| anyhow!("command output: unwrap failed"))?
    .read_to_string(buf)?;

  Ok(())
}
