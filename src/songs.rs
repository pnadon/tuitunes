use std::{path::PathBuf, error::Error, io::{BufReader, Cursor}, fs::File, env::temp_dir};

use rodio::{OutputStreamHandle, Sink, OutputStream};

use anyhow::anyhow;

use crate::spectrum::Analyzer;

pub fn load_app_and_sink<'a>(
  song: &'a PathBuf,
  stream_handle: &OutputStreamHandle,
) -> Result<(Analyzer<'a>, Sink), Box<dyn Error>> {
  if !has_supported_extension(song) {
    return Err(anyhow!("file {} is not a supported format", song.to_str().unwrap()).into());
  }
  let sink = stream_handle.play_once(BufReader::new(File::open(song)?))?;
  let app = crate::spectrum::Analyzer::new(crate::get_source::<f32, _>(song)?);

  Ok((app, sink))
}

fn has_supported_extension(path: &PathBuf) -> bool {
  crate::SUPPORTED_FORMATS
    .iter()
    .any(|ext| path.extension().and_then(|e| e.to_str()) == Some(*ext))
}

pub fn to_song_names<'a>(paths: &[PathBuf], rev: bool) -> Vec<&str> {
    let p = paths
      .iter()
      .map(|b| b.file_name().unwrap().to_str().unwrap());
    if rev {
      p.rev().collect::<Vec<&str>>()
    } else {
      p.collect::<Vec<&str>>()
    }
}

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

/// Unused, errors out with "NoDevice"
pub fn get_default_output_handle() -> OutputStreamHandle {
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();
  stream_handle
}
