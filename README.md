# TuiTunes

Experimental cli-based music player, with a waveform visualizer.

This isn't ready by any means, use at your own risk. This is just a result of me playing around with a fun project.

Usage (need Rust installed):

```
cargo run -- --path='path-to-audio' --color
```

`path-to-audio` can be a local file, a file accessible via HTTP, or a directory of files.
Supported formats are mp3, ogg, flac, wav, aac.
