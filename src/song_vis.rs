
use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::OutputStream;
use spectrum_analyzer::scaling::divide_by_N;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

use std::fs::{File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::{
  error::Error,
  io,
  time::{Duration, Instant},
};
use tui::{
  backend::{Backend, CrosstermBackend},
  layout::{Constraint, Direction, Layout},
  style::{Color, Style},
  widgets::{BarChart, Block, Borders},
  Frame, Terminal,
};

const NUM_BARS: usize = 64;
const TICK_RATE: u64 = 50;
const HANN_WINDOW_SIZE: usize = 2048;

struct App<'a> {
  sample_rate: u32,
  channels: u64,
  buf: Vec<f32>,
  source: Box<dyn rodio::Source<Item = f32> + Send + 'static>,
  data: Vec<(&'a str, f32)>,
}

impl<'a> App<'a> {
  pub fn new<S>(source: S) -> App<'a>
  where
    S: rodio::Source<Item = f32> + Send + 'static,
  {
    App {
      channels: source.channels() as u64,
      sample_rate: source.sample_rate() as u32,
      buf: vec![0.0; TICK_RATE as usize * 4 * source.sample_rate() as usize / 1000],
      source: Box::new(source),
      data: vec![("", 0.0); NUM_BARS],
    }
  }

  fn on_tick(&mut self, elapsed: u32) {
    let num_samples = (self.sample_rate * elapsed / 1000) as usize;
    let buf = &mut self.buf[0..HANN_WINDOW_SIZE];
    for i in 0..num_samples {
      let data = self.source.next().unwrap_or_default();
      if i < HANN_WINDOW_SIZE {
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

    self.data = vec![("", 0.0); NUM_BARS];
    for (fr, fr_val) in spectrum_hann_window.data().iter() {
      let bar = (fr.val() - 40.0) * NUM_BARS as f32 / (5000.0 - 40.0);
      self.data[bar as usize].1 += fr_val.val()
    }

    // dbg!(val, val.saturating_sub(self.min));
  }
}

pub fn run(song_path: &str) -> Result<(), Box<dyn Error>> {
  // setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let res = run_app(&mut terminal, song_path);

  // restore terminal
  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    DisableMouseCapture
  )?;
  terminal.show_cursor()?;

  if let Err(err) = res {
    println!("{:?}", err)
  }

  Ok(())
}

fn run_app<B: Backend>(
  terminal: &mut Terminal<B>,
  song_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();

  let tick_rate = Duration::from_millis(TICK_RATE);

  let song_path = Path::new(song_path);
  let songs = {
    let mut s = if song_path.is_dir() {
      song_path
        .read_dir()?
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().unwrap().is_file())
        .map(|e| e.path())
        .collect::<Vec<PathBuf>>()
    } else {
      vec![song_path.to_owned()]
    };
    s.sort();
    s
  };
  for song in songs.iter() {
    let mut app = App::new(crate::get_source::<f32, _>(song)?);
    // Play the sound directly on the device
    let mut sink = stream_handle.play_once(BufReader::new(File::open(song)?))?;

    let mut last_tick = Instant::now();
    let song_name = song.file_name().unwrap();
    'song: loop {
      terminal.draw(|f| ui(f, &app, song_name.to_str().unwrap()))?;

      let timeout = tick_rate
        .checked_sub(last_tick.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));
      if crossterm::event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
          match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char('n') => {
              break 'song;
            }
            KeyCode::Char('p') => {
              if sink.is_paused() {
                sink.play();
                last_tick = Instant::now();
              } else {
                sink.pause();
              }
            }
            KeyCode::Char('r') => {
              sink.stop();
              app = App::new(crate::get_source::<f32, _>(song)?);
              sink = stream_handle.play_once(BufReader::new(File::open(song)?))?;
            }
            _ => (),
          }
        }
      }
      if sink.empty() {
        break 'song;
      }
      if !sink.is_paused() && last_tick.elapsed() >= tick_rate {
        let elapsed = last_tick.elapsed().as_millis();
        last_tick = Instant::now();
        app.on_tick(elapsed as u32);
      }
    }
  }
  Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App, song_name: &str) {
  let data = app
    .data
    .iter()
    .map(|(_, v)| ("", (v * 1000.0) as u64 + 10))
    .collect::<Vec<(&str, u64)>>();
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(2)
    .constraints([Constraint::Length(NUM_BARS as u16 * 2)].as_ref())
    .split(f.size());
  let barchart = BarChart::default()
    .block(
      Block::default()
        .title(format!("now-playing:-{}", song_name))
        .borders(Borders::ALL),
    )
    .data(&data)
    .bar_width(2)
    .bar_gap(0)
    .bar_style(Style::default().fg(Color::Yellow));
  f.render_widget(barchart, chunks[0]);
}
