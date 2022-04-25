use std::{fs::File, io::BufReader, path::Path};

use rodio::{source::SamplesConverter, Decoder, Source};

const NUM_BARS: usize = 48;
const TICK_RATE: u64 = 50;
const HANN_WINDOW_SIZE: usize = 2048;

const SUPPORTED_FORMATS: [&str; 5] = ["mp3", "flac", "ogg", "wav", "aac"];

pub mod app;
pub mod songs;
pub mod spectrum;
pub mod ui;

pub fn get_source<D: rodio::Sample, P: AsRef<Path>>(
  song_path: P,
) -> Result<SamplesConverter<Decoder<BufReader<File>>, D>, Box<dyn std::error::Error>> {
  // Load a sound from a file, using a path relative to Cargo.toml
  let file = BufReader::new(File::open(song_path)?);
  // Decode that sound file into a source
  Ok(Decoder::new(file)?.convert_samples::<D>())
}
