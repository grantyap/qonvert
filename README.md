# Qonvert

A tiny CLI for batch video conversion.

Trying to send a bunch of WEBMs to someone else but your messaging app attaches them as files? Or maybe you wanted to convert funny GIFs into videos to save space on your device? Qonvert is a wrapper around FFmpeg written in Rust that batch converts any input files accepted by FFmpeg into video files.

> [!NOTE]
> Currently, only the `mp4` file type and `libx265` and `hevc_videotoolbox` codecs are supported for output files.

## Installation

```sh
$ cargo install --git git@github.com:grantyap/qonvert.git
```

## Usage

To convert video files and export them into MP4 files with the libx265 codec inside the current directory, run:

```sh
# Outputs video1.mp4 and video2.mp4 in the current directory.
$ qo video1.webm video2.webm -t mp4 -c libx265
```

For more detailed usage, run:

```sh
$ qo --help
```
