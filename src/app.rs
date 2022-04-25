use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{OutputStream, OutputStreamHandle};

use crate::songs::{get_search_dir, load_app_and_sink, load_song_list, to_song_names};
use crate::ui::{add_songs_popup, get_ui_color, main_ui};
use std::path::PathBuf;
use std::str::FromStr;
use std::{
  error::Error,
  io,
  time::{Duration, Instant},
};
use tui::{
  backend::{Backend, CrosstermBackend},
  Terminal,
};

/// Sets up the terminal, and runs the UI.
pub fn run(song_path: PathBuf, use_default_color: bool) -> Result<(), Box<dyn Error>> {
  // setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();

  // run application
  let res = run_app(&mut terminal, stream_handle, song_path, use_default_color);

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

/// Runs the UI loop, assuming the terminal has been prepared.
fn run_app<B: Backend>(
  terminal: &mut Terminal<B>,
  stream_handle: OutputStreamHandle,
  song_path: PathBuf,
  use_default_color: bool,
) -> Result<(), Box<dyn std::error::Error>> {
  let tick_rate = Duration::from_millis(crate::TICK_RATE);

  let mut songs = load_song_list(song_path)?;
  let mut history: Vec<PathBuf> = vec![];

  while !songs.is_empty() {
    let song = songs.pop().unwrap();

    let maybe_song_data = load_app_and_sink(&song, &stream_handle);
    if let Err(e) = &maybe_song_data {
      eprintln!("could not load song, skipping...: {}", e);
      continue; // skip to next song
    }
    let (mut analyzer, mut sink) = maybe_song_data.unwrap();

    let song_name = song.file_name().unwrap().to_str().unwrap();
    let ui_color = get_ui_color(song_name, use_default_color);
    let mut last_tick = Instant::now();
    'song: loop {
      terminal.draw(|f| {
        main_ui(
          f,
          &analyzer,
          song_name,
          &to_song_names(&songs, true),
          &to_song_names(&history, false),
          ui_color,
        )
      })?;

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
              (analyzer, sink) = load_app_and_sink(&song, &stream_handle)?
            }
            KeyCode::Char('a') => {
              sink.pause();
              let mut buf = get_search_dir();
              'add_songs: loop {
                terminal.draw(|f| add_songs_popup(f, &buf, ui_color))?;
                if let Event::Key(k) = event::read()? {
                  match k.code {
                    KeyCode::Esc => {
                      sink.play();
                      last_tick = Instant::now();
                      break 'add_songs;
                    }
                    KeyCode::Enter => {
                      let mut new_song_list = load_song_list(PathBuf::from_str(&buf)?)?;
                      new_song_list.append(&mut songs);
                      songs = new_song_list;
                      songs.push(song);
                      break 'song;
                    }
                    KeyCode::Backspace => {
                      buf.pop();
                    }
                    KeyCode::Char(c) => {
                      buf.push(c);
                    }
                    _ => (),
                  }
                }
              }
            }
            KeyCode::Char('s') => {
              songs.push(song);
              fastrand::shuffle(&mut songs);
              break 'song;
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
        analyzer.sample_audio(elapsed as u32);
      }
    }
  }
  Ok(())
}
