use rodio::{source::Source, OutputStream};
use std::error::Error;

use std::io;

use crate::{get_source, TEST_SONG_PATH};

pub fn run() -> Result<(), Box<dyn Error>> {
  // Get a output stream handle to the default physical sound device
  let (_stream, stream_handle) = OutputStream::try_default()?;

  let source = get_source(TEST_SONG_PATH)?;

  dbg!(
    source.current_frame_len(),
    source.channels(),
    source.sample_rate(),
    source.total_duration()
  );
  // Play the sound directly on the device
  stream_handle.play_raw(source)?;

  println!("Press x followed by enter to exit.");

  // The sound plays in a separate audio thread,
  // so we need to keep the main thread alive while it's playing.
  let mut buffer = String::new();
  while buffer.trim() != "x" {
    buffer.clear();
    io::stdin().read_line(&mut buffer)?;
  }

  Ok(())
}
