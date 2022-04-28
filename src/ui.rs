use std::{collections::hash_map::DefaultHasher, hash::Hasher};

use tui::{
  backend::Backend,
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  widgets::{self, BarChart, Block, Borders, Clear, List, ListItem, Paragraph},
  Frame,
};

use crate::spectrum::Analyzer;

/// The main UI that the user sees.
pub fn main_ui<B: Backend>(
  f: &mut Frame<B>,
  analyzer: &Analyzer,
  song_name: &str,
  up_next: &[&str],
  history: &[&str],
  ui_color: Color,
) {
  let data = analyzer
    .get_spectrum()
    .iter()
    .map(|(_, v)| ("", (v * 1000.0) as u64 + 10))
    .collect::<Vec<(&str, u64)>>();

  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(0)
    .constraints([Constraint::Min(10), Constraint::Percentage(70)].as_ref())
    .split(f.size());

  let visualizer_chunk = Layout::default()
    .direction(Direction::Horizontal)
    .margin(1)
    .constraints(
      [
        Constraint::Min((crate::NUM_BARS * 2) as u16),
        Constraint::Percentage(60),
      ]
      .as_ref(),
    )
    .split(chunks[0]);

  let lists_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .margin(1)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
    .split(chunks[1]);

  f.render_widget(spectrum_visualizer(&data, ui_color), visualizer_chunk[0]);
  f.render_widget(now_playing(song_name, ui_color), visualizer_chunk[1]);
  f.render_widget(up_next_list(up_next, ui_color), lists_chunks[0]);
  f.render_widget(history_list(history, ui_color), lists_chunks[1]);
}

/// Displays the spectrum visualizer for the currently playing song.
fn spectrum_visualizer<'a>(data: &'a [(&str, u64)], ui_color: Color) -> BarChart<'a> {
  BarChart::default()
    .block(Block::default().title("tuitunes").borders(Borders::ALL))
    .style(Style::default().fg(ui_color))
    .data(data)
    .bar_width(2)
    .bar_gap(0)
    .bar_style(Style::default().fg(ui_color))
}

/// Displays info for the currently playing song, as well as controls.
fn now_playing(song_name: &str, ui_color: Color) -> Paragraph {
  Paragraph::new(format!(
    "{song_name}\n\nq: quit\nn: next\nb: back\np: play/pause\nr: restart song\na: add songs\ns: shuffle"
    ))
    .block(
      Block::default()
        .title("now-playing")
        .borders(Borders::ALL)
    )
    .style(Style::default().fg(ui_color))
}

/// Displays the list of upcoming songs.
fn up_next_list<'a>(up_next: &'a [&str], ui_color: Color) -> List<'a> {
  List::new(
    up_next
      .iter()
      .map(|s| ListItem::new(*s))
      .collect::<Vec<ListItem>>(),
  )
  .block(Block::default().title("up-next").borders(Borders::ALL))
  .style(Style::default().fg(ui_color))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
}

/// Displays the list of songs which have already played.
fn history_list<'a>(history: &'a [&str], ui_color: Color) -> List<'a> {
  List::new(
    history
      .iter()
      .map(|s| ListItem::new(*s))
      .collect::<Vec<ListItem>>(),
  )
  .block(Block::default().title("history").borders(Borders::ALL))
  .style(Style::default().fg(ui_color))
  .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
}

/// Creates a rectangle centered in the middle of the terminal.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
  let popup_layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints(
      [
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
      ]
      .as_ref(),
    )
    .split(r);

  Layout::default()
    .direction(Direction::Horizontal)
    .constraints(
      [
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
      ]
      .as_ref(),
    )
    .split(popup_layout[1])[1]
}

/// Displays the popup which lets users add more songs.
pub fn add_songs_popup<B: Backend>(
  f: &mut Frame<B>,
  text: &str,
  search_res: &str,
  ui_color: Color,
) {
  let search_input = widgets::Paragraph::new(text)
    .block(
      Block::default()
        .title("enter-path-to-songs")
        .borders(Borders::ALL),
    )
    .style(Style::default().fg(ui_color));
  let area = centered_rect(60, 60, f.size());

  let search_results = widgets::Paragraph::new(search_res)
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(ui_color).add_modifier(Modifier::ITALIC));

  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
    .split(area);

  f.render_widget(Clear, area);
  f.render_widget(search_input, layout[0]);
  f.render_widget(search_results, layout[1]);
}

/// Gets the color of the UI.
/// If use_default is set then it just uses yellow.
/// Otherwise, it chooses a color by computing the hash of the song's name.
pub fn get_ui_color(song_name: &str, use_default: bool) -> Color {
  if !use_default {
    let mut s = DefaultHasher::new();
    s.write(song_name.as_bytes());
    Color::Indexed((s.finish() % 15) as u8 + 1)
  } else {
    Color::Yellow
  }
}
