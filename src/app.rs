use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{OutputStream, OutputStreamHandle};

use crate::songs::{
  get_search_dir, load_app_and_sink, load_song_list, search_songs, to_song_names,
};
use crate::ui::{add_songs_popup, get_ui_color, main_ui};
use std::{path::PathBuf, env};
use std::str::FromStr;
use std::{
  error::Error,
  io,
  time::{Duration, Instant},
};
use tui::{
  backend::{Backend, CrosstermBackend},
  Terminal, style::Color,
};

use anyhow::anyhow;

/// Sets up the terminal, and runs the UI.
pub fn run(song_path: Option<PathBuf>, use_default_color: bool) -> Result<(), Box<dyn Error>> {
  let config_dir = format!("{}/.config/tuitunes/", env::var("HOME")?);
  let config = format!("{}songs.txt", config_dir);
  
  let mut history: Vec<PathBuf> = vec![];
  let mut play_next = match song_path {
    Some(p) => load_song_list(p)?,
    None => {
      std::fs::create_dir_all(&config_dir)?;
      match std::fs::read_to_string(&config) {
        Ok(s) => {
          s.split("\n")
            .filter(|s| !s.trim().is_empty())
            .map(|s| PathBuf::from(s))
            .collect::<Vec<PathBuf>>()
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
          std::fs::File::create(&config)?; 
          vec![]
        }
        _ => {return Err(anyhow!("No path was provided, and failed to load any songs from config").into());}
      }
    }
  };

  // setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();
  
  // run application
  let res = run_app(&mut terminal, stream_handle, &mut play_next, &mut history, use_default_color);

  // restore terminal
  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    DisableMouseCapture
  )?;
  terminal.show_cursor()?;

  if !play_next.is_empty() && res.is_ok() {
    if let Some(p) = play_next.iter().map(|s| s.to_str()).collect::<Option<Vec<&str>>>() {
      Ok(std::fs::write(config, p.join("\n"))?)
    } else {
      println!("invalid paths");
      res
    }
  } else {
    println!("nothing to write");
    res
  }
}

/// Runs the UI loop, assuming the terminal has been prepared.
fn run_app<B: Backend>(
  terminal: &mut Terminal<B>,
  stream_handle: OutputStreamHandle,
  play_next: &mut Vec<PathBuf>,
  history: &mut Vec<PathBuf>,
  use_default_color: bool,
) -> Result<(), Box<dyn std::error::Error>> {
  let tick_rate = Duration::from_millis(crate::TICK_RATE);

  loop {
    if play_next.is_empty() {
      match submit_more_songs(terminal, crate::ui::DEFAULT_COLOR)? {
        Some(p) => {
          let mut more_songs = load_song_list(PathBuf::from(p))?;
          if more_songs.is_empty() {
            return Ok(())
          }
          play_next.append(&mut more_songs);
        },
        None => return Ok(())
      }
    }
    let song = play_next.pop().unwrap();

    let maybe_song_data = load_app_and_sink(&song, &stream_handle);
    if let Err(e) = &maybe_song_data {
      eprintln!("could not load song, skipping...: {}", e);
      continue; // skip to next song
    }
    let (mut analyzer, mut sink) = maybe_song_data.unwrap();

    let song_name = song.file_stem().unwrap().to_str().unwrap();
    let ui_color = get_ui_color(song_name, use_default_color);
    let mut last_tick = Instant::now();
    'song: loop {
      terminal.draw(|f| {
        main_ui(
          f,
          &analyzer,
          song_name,
          &to_song_names(&play_next, true),
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
              play_next.push(song);
              if let Some(s) = history.pop() {
                play_next.push(s);
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
              match submit_more_songs(terminal, ui_color)? {
                Some(buf) => {
                  let mut new_song_list = load_song_list(PathBuf::from_str(&buf)?)?;
                  new_song_list.append(play_next);
                  *play_next = new_song_list;
                  play_next.push(song);
                  break 'song;
                }
                None => {
                  sink.play();
                  last_tick = Instant::now();
                }
              };
            }
            KeyCode::Char('s') => {
              play_next.push(song);
              fastrand::shuffle(play_next);
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
}

fn submit_more_songs<B: Backend>(terminal: &mut Terminal<B>, ui_color: Color) -> Result<Option<String>, Box<dyn Error>> {
  let mut buf = get_search_dir();
  let mut res_buf = String::new();
  loop {
    res_buf.clear();
    search_songs(&buf, &mut res_buf)?;
    terminal.draw(|f| add_songs_popup(f, &buf, &res_buf, ui_color))?;
    if let Event::Key(k) = event::read()? {
      match k.code {
        KeyCode::Esc => {
          return Ok(None);
        }
        KeyCode::Enter => {
          return Ok(Some(buf));
        }
        KeyCode::Backspace => {
          buf.pop();
        }
        KeyCode::Tab => {
          if let Some(s) = res_buf.split('\n').find(|s| !s.is_empty()) {
            buf = s.to_owned();
          }
        }
        KeyCode::Char(c) => {
          buf.push(c);
        }
        _ => (),
      }
    }
  }
}
