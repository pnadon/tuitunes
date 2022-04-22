use std::{fs::File, io::BufReader, path::Path};

use rodio::{source::SamplesConverter, Decoder, Source};

pub mod song_vis;

pub fn get_source<D: rodio::Sample, P: AsRef<Path>>(
  song_path: P,
) -> Result<SamplesConverter<Decoder<BufReader<File>>, D>, Box<dyn std::error::Error>> {
  // Load a sound from a file, using a path relative to Cargo.toml
  let file = BufReader::new(File::open(song_path)?);
  // Decode that sound file into a source
  Ok(Decoder::new(file)?.convert_samples::<D>())
}
