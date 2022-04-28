const NUM_BARS: usize = 48;
const TICK_RATE: u64 = 50;
const HANN_WINDOW_SIZE: usize = 2048;

const SUPPORTED_FORMATS: [&str; 5] = ["mp3", "flac", "ogg", "wav", "aac"];

pub mod app;
pub mod search;
pub mod songs;
pub mod spectrum;
pub mod ui;
