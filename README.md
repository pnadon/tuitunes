# TuiTunes

Experimental cli-based music player, with a waveform visualizer.

This isn't ready by any means, use at your own risk. This is just a result of me playing around with a fun project.

There aren't any tests (I'm sorry).

The search functionality relies on `fd`, so you will need that installed. I'm currently looking at using a pure Rust library for search so that installing `fd` won't be necessary.

## How to launch (need Rust installed):

```
cargo run -- --path='path-to-audio' --color
```

`path-to-audio` can be a local file, a file accessible via HTTP, or a directory of files.
Supported formats are mp3, ogg, flac, wav, aac.

If you provide no path, it will either restore your previous session, or if you just launched it for the first time, it will prompt you to add songs.

## Interacting with the UI

### q: quit
Quit the app.

### n: next
Play the next song.

### b: back
Play the previous song.

### p: play/pause
Toggle playing / pausing the song.

### r: restart song
Start playing the current song back at the beginning.

### a: add songs
Opens a popup that lets you submit a path to a (local) song.
If `MUSIC_HOME` is set, it will pre-populate the search with that directory.
Otherwise, it will try `HOME`, and lastly default to an empty query to start with.

Search suggestions will appear below, you can press TAB to populate your search with the first one.
### s: shuffle"
Randomizes the order of the songs.
