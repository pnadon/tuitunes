use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use spectrum_analyzer::scaling::divide_by_N;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

use anyhow::anyhow;
use std::io::BufReader;
use std::path::PathBuf;
use std::{collections::hash_map::DefaultHasher, fs::File, hash::Hasher};
use std::{
  error::Error,
  io,
  time::{Duration, Instant},
};
use tui::{
  backend::{Backend, CrosstermBackend},
  layout::{Constraint, Direction, Layout},
  style::{Color, Modifier, Style},
  widgets::{BarChart, Block, Borders, List, ListItem, self},
  Frame, Terminal,
};

const NUM_BARS: usize = 48;
const TICK_RATE: u64 = 50;
const HANN_WINDOW_SIZE: usize = 2048;

const SUPPORTED_FORMATS: [&str; 5] = ["mp3", "flac", "ogg", "wav", "aac"];

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

pub fn run(song_path: PathBuf, color: bool) -> Result<(), Box<dyn Error>> {
  // setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let res = run_app(&mut terminal, song_path, color);

  // restore terminal
  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    DisableMouseCapture
  )?;
  terminal.show_cursor()?;

  res
}

fn run_app<B: Backend>(
  terminal: &mut Terminal<B>,
  song_path: PathBuf,
  color: bool,
) -> Result<(), Box<dyn std::error::Error>> {
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();

  let tick_rate = Duration::from_millis(TICK_RATE);

  let mut songs = {
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
    s
  };
  let mut history: Vec<PathBuf> = vec![];

  while !songs.is_empty() {
    let song = songs.pop().unwrap();
    let maybe_song_data = load_app_and_sink(&song, &stream_handle);
    if let Err(e) = &maybe_song_data {
      eprintln!("could not load song, skipping...: {}", e);
      continue;
    }
    let (mut app, mut sink) = maybe_song_data.unwrap();

    let mut last_tick = Instant::now();
    let song_name = song.file_name().unwrap().to_str().unwrap();

    let up_next = song_list(&songs, true);
      
    let ui_color = if color {
      let mut s = DefaultHasher::new();
      s.write(song_name.as_bytes());
      Color::Indexed((s.finish() % 15) as u8 + 1)
    } else {
      Color::Yellow
    };
    'song: loop {
      terminal.draw(|f| ui(f, &app, song_name, &up_next, &song_list(&history, false), ui_color))?;

      let timeout = tick_rate
        .checked_sub(last_tick.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));
      if crossterm::event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
          match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char('n') => {
              history.push(song);
              break 'song;
            }
            KeyCode::Char('b') => {
              songs.push(song);
              if let Some(s) = history.pop() {
                songs.push(s);
              }
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
              app = App::new(crate::get_source::<f32, _>(&song)?);
              sink = stream_handle.play_once(BufReader::new(File::open(&song)?))?;
            }
            _ => (),
          }
        }
      }
      if sink.empty() {
        history.push(song);
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

fn ui<B: Backend>(f: &mut Frame<B>, app: &App, song_name: &str, up_next: &[&str], history: &[&str], ui_color: Color) {
  let data = app
    .data
    .iter()
    .map(|(_, v)| ("", (v * 1000.0) as u64 + 10))
    .collect::<Vec<(&str, u64)>>();
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(0)
    .constraints(
      [
        Constraint::Min(10),
        Constraint::Percentage(70),
      ]
      .as_ref(),
    )
    .split(f.size());
  
  let visualizer_chunk = Layout::default()
    .direction(Direction::Horizontal)
    .margin(1)
    .constraints([
      Constraint::Min((NUM_BARS * 2) as u16),
      Constraint::Percentage(60),
    ].as_ref())
    .split(chunks[0]);
  
  let lists_chunks = Layout::default()
    .direction(Direction::Horizontal)
      .margin(1)
      .constraints(
        [
          Constraint::Percentage(50),
          Constraint::Percentage(50),
        ]
        .as_ref(),
      )
      .split(chunks[1]);

  let barchart = BarChart::default()
    .block(
      Block::default()
        .title("tuitunes")
        .borders(Borders::ALL),
    )
    .style(Style::default().fg(ui_color))
    .data(&data)
    .bar_width(2)
    .bar_gap(0)
    .bar_style(Style::default().fg(ui_color));

  let now_playing = widgets::Paragraph::new(format!(
    "Now playing:\n{song_name}\n\nq: quit\nn: next\nb: back\np: play/pause\nr: restart song"
    ))
    .block(
      Block::default()
        .borders(Borders::ALL)
    )
    .style(Style::default().fg(ui_color));

  let up_next_list = List::new(
    up_next
      .iter()
      .map(|s| ListItem::new(*s))
      .collect::<Vec<ListItem>>(),
  )
  .block(Block::default().title("up-next").borders(Borders::ALL))
  .style(Style::default().fg(ui_color))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC));

  let history_list = List::new(
    history
      .iter()
      .map(|s| ListItem::new(*s))
      .collect::<Vec<ListItem>>(),
  )
  .block(Block::default().title("history").borders(Borders::ALL))
  .style(Style::default().fg(ui_color))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC));

  f.render_widget(barchart, visualizer_chunk[0]);
  f.render_widget(now_playing, visualizer_chunk[1]);
  f.render_widget(up_next_list, lists_chunks[0]);
  f.render_widget(history_list, lists_chunks[1]);
}

fn load_app_and_sink<'a>(
  song: &'a PathBuf,
  stream_handle: &OutputStreamHandle,
) -> Result<(App<'a>, Sink), Box<dyn Error>> {
  if !has_supported_extension(song) {
    return Err(anyhow!("file {} is not a supported format", song.to_str().unwrap()).into());
  }
  let sink = stream_handle.play_once(BufReader::new(File::open(song)?))?;
  let app = App::new(crate::get_source::<f32, _>(song)?);

  Ok((app, sink))
}

fn has_supported_extension(path: &PathBuf) -> bool {
  SUPPORTED_FORMATS
    .iter()
    .any(|ext| path.extension().and_then(|e| e.to_str()) == Some(*ext))
}

fn song_list<'a>(paths: &[PathBuf], rev: bool) -> Vec<&str> {
    let p = paths
      .iter()
      .map(|b| b.file_name().unwrap().to_str().unwrap());
    if rev {
      p.rev().collect::<Vec<&str>>()
    } else {
      p.collect::<Vec<&str>>()
    }
}
