# omvdecoder

* decoding SiglusEngine's omv video, support both 32bit 'shader video' and 24bit normal video.
* cross-platform, support Windows, Linux, MacOS.
* can be decoded to h264, png, jpg.

## Build
* install rust: https://www.rust-lang.org/tools/install
* clone this repo
* run `cargo build --release`

## Usage

```bash
Usage: omvdecoder --input <INPUT> --output <OUTPUT> --format <FORMAT>

Options:
  -i, --input <INPUT>    
  -o, --output <OUTPUT>  
  -f, --format <FORMAT>  [possible values: h264, png-picture, jpg-picture]
  -h, --help             Print help
  -V, --version          Print version
```

* for ffmpeg mode, you need to install ffmpeg first, and make sure ffmpeg is in your PATH.
* ffmpeg mode can guarantee higher quality when compared to the h264 mode.
