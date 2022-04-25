use spectrum_analyzer::scaling::divide_by_N;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

pub struct Analyzer<'a> {
  sample_rate: u32,
  channels: u64,
  buf: Vec<f32>,
  source: Box<dyn rodio::Source<Item = f32> + Send + 'static>,
  data: Vec<(&'a str, f32)>,
}

impl<'a> Analyzer<'a> {
  pub fn new<S>(source: S) -> Analyzer<'a>
  where
    S: rodio::Source<Item = f32> + Send + 'static,
  {
    Analyzer {
      channels: source.channels() as u64,
      sample_rate: source.sample_rate() as u32,
      buf: vec![0.0; crate::TICK_RATE as usize * 4 * source.sample_rate() as usize / 1000],
      source: Box::new(source),
      data: vec![("", 0.0); crate::NUM_BARS],
    }
  }

  pub fn on_tick(&mut self, elapsed: u32) {
    let num_samples = (self.sample_rate * elapsed / 1000) as usize;
    let buf = &mut self.buf[0..crate::HANN_WINDOW_SIZE];
    for i in 0..num_samples {
      let data = self.source.next().unwrap_or_default();
      if i < crate::HANN_WINDOW_SIZE {
        buf[i] = data
      }
      for _ in 0..self.channels - 1 {
        self.source.next();
      }
    }
    let hann_window = hann_window(buf);
    // calc spectrum
    let spectrum_hann_window = samples_fft_to_spectrum(
      // (windowed) samples
      &hann_window,
      // sampling rate
      self.sample_rate,
      // optional frequency limit: e.g. only interested in frequencies 50 <= f <= 150?
      FrequencyLimit::Range(40.0, 5000.0),
      // optional scale
      Some(&divide_by_N),
    )
    .unwrap();

    self.data = vec![("", 0.0); crate::NUM_BARS];
    for (fr, fr_val) in spectrum_hann_window.data().iter() {
      let bar = (fr.val() - 40.0) * crate::NUM_BARS as f32 / (5000.0 - 40.0);
      self.data[bar as usize].1 += fr_val.val()
    }
  }

  pub fn data(&self) -> &[(&'a str, f32)] {
    &self.data
  }
}
