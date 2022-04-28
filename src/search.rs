use anyhow::anyhow;
use std::{
  env,
  error::Error,
  io::{Cursor, Read},
  path::PathBuf,
  process::{Child, Command, Stdio},
};

use skim::prelude::*;

pub fn run_search(options: SkimOptions, reader: SkimItemReader, query: String) -> Vec<String> {
  let items = reader.of_bufread(Cursor::new(query));
  let selected_items = Skim::run_with(&options, Some(items))
    .map(|out| out.selected_items)
    .unwrap_or_else(Vec::new);

  selected_items
    .iter()
    .map(|s| s.text().to_string())
    .collect::<Vec<String>>()
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
  let mut command: Child = Command::new(shell)
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
