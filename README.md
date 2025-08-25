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


## Piped PNG Stream Protocol

`omvdecoder` does **not** write files. It streams frames to **stdout** so a parent process can consume them via a pipe. Inspired by Y4M.

### Overview

* **Transport:** stdout (cross-platform pipe).
* **Encoding per frame:** PNG (RGBA8).
* **Line endings:** `\n` (LF). Control lines are UTF-8 text.
* **No extra blank lines.** Binary data follows immediately after the `BYTES` line.

### Stream layout

1. **Header (single line):**

   ```
   PXY4M W{width} H{height} F{num}/{den} Crgba Enc:png
   ```

   * `W/H`: frame size in pixels
   * `F`: frame rate as a rational number `num/den`
   * `Crgba`: input pixel format (what the encoder received)
   * `Enc:png`: per-frame payload is PNG

2. **Repeated per frame:**

   ```
   FRAME
   PTS {pts}
   BYTES {n}
   <n raw bytes of PNG immediately here>
   ```

   * `PTS` is a monotonically increasing integer. By default it equals the provided `frame_id`.
   * `BYTES {n}` tells the consumer exactly how many bytes to read for the PNG payload.

3. **End of stream:** EOF (process exit or pipe closed). No footer.

### Example

```
PXY4M W640 H360 F30000/1001 Crgba Enc:png
FRAME
PTS 0
BYTES 123456
<123456 bytes of PNG>
FRAME
PTS 1
BYTES 123987
<123987 bytes of PNG>
...
```

### Consumer guidance

* Read and parse the single **header** line first to obtain `width`, `height`, and frame rate.

* For each frame:

  1. Read the literal line `FRAME`.
  2. Read `PTS {pts}` and parse `{pts}` as an integer.
  3. Read `BYTES {n}`, parse `{n}` as a non-negative integer.
  4. **Read exactly `n` bytes** from the stream for the PNG payload. Do **not** assume a single `read()` returns all bytes—loop until `n` bytes are collected.
  5. Decode the PNG with any standard PNG library. Dimensions should match the header’s `W`/`H`; color mode is RGBA8.

* See `test.py` for a minimal Python consumer example.

### Notes & tips

* Unix/WSL/macOS:

  ```bash
  ./omvdecoder input.mp4 | python consumer.py
  ```
* Windows (PowerShell):

  ```powershell
  .\omvdecoder.exe input.mp4 | python .\consumer.py
  ```

