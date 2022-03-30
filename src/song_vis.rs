use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::OutputStream;
use std::{
  error::Error,
  io,
  time::{Duration, Instant}, thread::sleep, cmp,
};
use tui::{
  backend::{Backend, CrosstermBackend},
  layout::{Constraint, Direction, Layout},
  style::{Color, Modifier, Style},
  widgets::{BarChart, Block, Borders},
  Frame, Terminal,
};
use anyhow::anyhow;

const NUM_BARS: usize = 256;
const TICK_RATE: u64 = 20;
const MIN_CROP: u64 = 22000;

struct App<'a> {
  min: u64,
  max: u64,
  sample_rate: u64,
  channels: u64,
  buf: Vec<u64>,
  source: Box<dyn rodio::Source<Item = u16> + Send + 'static>,
  data: Vec<(&'a str, u64)>,
}

impl<'a> App<'a> {
  pub fn new<S>(source: S) -> App<'a>
  where
    S: rodio::Source<Item = u16> + Send + 'static,
  {
    App {
      max: 0,
      min: u16::MAX as u64,
      channels: source.channels() as u64,
      sample_rate: source.sample_rate() as u64,
      buf: vec![0; TICK_RATE as usize * 2 * source.sample_rate() as usize / 1000],
      source: Box::new(source.into_iter()),
      data: vec![("", 0); NUM_BARS],
    }
  }

  fn on_tick(&mut self, elapsed: u64) {
    let num_samples = (self.sample_rate * elapsed / 1000) as usize;
    let buf = &mut self.buf[0..num_samples];
    for i in 0..num_samples {
      buf[i] = self.source.next().unwrap_or_default() as u64;
      for _ in 0..self.channels - 1 {
        self.source.next();
      }
    }
    let val = buf.iter().sum::<u64>() / buf.len() as u64;
    self.data.pop().unwrap();
    self.min = cmp::min(self.min, val);
    self.max = cmp::max(self.max, val);
    self
      .data
      .insert(0, ("", val.saturating_sub(self.min)));
    
    // dbg!(val, val.saturating_sub(self.min));
  }
}

pub fn run(song_path: &str, no_brightness: bool) -> Result<(), Box<dyn Error>> {
  // setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // create app and run it
  let tick_rate = Duration::from_millis(TICK_RATE);
  let app = App::new(crate::get_source::<u16>(song_path)?);
  let res = run_app(&mut terminal, app, song_path, no_brightness, tick_rate);

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
  mut app: App,
  song_path: &str,
  no_brightness: bool,
  tick_rate: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
  // let handler = std::thread::spawn(|| {
  // Get a output stream handle to the default physical sound device
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();

  let source = crate::get_source(song_path).unwrap();
  // Play the sound directly on the device
  stream_handle.play_raw(source).unwrap();

  let mut last_tick = Instant::now();
  loop {
    terminal.draw(|f| ui(f, &app, no_brightness))?;

    let timeout = tick_rate
      .checked_sub(last_tick.elapsed())
      .unwrap_or_else(|| Duration::from_secs(0));
    if crossterm::event::poll(timeout)? {
      if let Event::Key(key) = event::read()? {
        if let KeyCode::Char('q') = key.code {
          return Ok(())
        }
      }
    }
    if last_tick.elapsed() >= tick_rate {
      let elapsed = last_tick.elapsed().as_millis();
      last_tick = Instant::now();
      app.on_tick(elapsed as u64);
    }
  }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App, no_brightness: bool) {
  let color = if no_brightness {
    Color::Yellow
  } else {
    // // need to think about this one...
    // let last_val = app.data.last().map(|v| v.1).unwrap_or(app.min);
    // let rgb_val = cmp::min(u8::MAX as u64 * last_val / (app.max.saturating_sub(app.min) + 1), u8::MAX as u64) as u8;
    // dbg!(rgb_val, last_val, (app.max.saturating_sub(app.min) + 1));
    // Color::Rgb(rgb_val, rgb_val, rgb_val)
    Color::Yellow
  };
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(2)
    .constraints([Constraint::Percentage(100)].as_ref())
    .split(f.size());
  let barchart = BarChart::default()
    .block(Block::default().title(format!("min:{}-----max:{}", app.min, app.max)).borders(Borders::ALL))
    .data(&app.data)
    .bar_width(1)
    .bar_gap(0)
    .bar_style(Style::default().fg(color))
    .value_style(Style::default().fg(Color::Black).bg(Color::Yellow));
  f.render_widget(barchart, chunks[0]);
}
