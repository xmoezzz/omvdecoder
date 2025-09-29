use std::{
    cell::RefCell,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
};

use anyhow::{anyhow, Result};
use image::RgbaImage;
use which::which;

use super::Converter;

pub struct FfmepgConverter {
    path: PathBuf,
    encoder: RefCell<Option<Child>>,
    stdin: RefCell<Option<ChildStdin>>,
    fps: f32,
    width: u32,
    height: u32,
}


impl FfmepgConverter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            encoder: RefCell::new(None),
            stdin: RefCell::new(None),
            fps: 0.0,
            width: 0,
            height: 0,
        }
    }
}

fn rgba_to_rgb(img: RgbaImage) -> Vec<u8> {
    let mut out = Vec::with_capacity((img.width() * img.height() * 3) as usize);
    for pixel in img.pixels() {
        out.extend_from_slice(&pixel.0[0..3]);
    }
    out
}

impl Converter for FfmepgConverter {
    fn prepare(&mut self, width: u32, height: u32, fps: f32) -> Result<()> {
        self.width = width;
        self.height = height;
        self.fps = fps;

        let ffmpeg_path = which("ffmpeg").map_err(|_| anyhow!("ffmpeg not found"))?;

        let mut cmd = Command::new(ffmpeg_path);
        cmd.arg("-y")
            .arg("-f").arg("rawvideo")
            .arg("-pix_fmt").arg("rgba")
            .arg("-s").arg(format!("{}x{}", width, height))
            .arg("-r").arg(format!("{}", fps))
            .arg("-i").arg("-")
            .arg("-c:v").arg("libx264")
            .arg("-pix_fmt").arg("yuv420p")
            .arg("-profile:v").arg("main")
            .arg("-crf").arg("18")
            .arg(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| anyhow!("Failed to spawn ffmpeg: {}", e))?;
        let child_stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;

        *self.encoder.borrow_mut() = Some(child);
        *self.stdin.borrow_mut() = Some(child_stdin);
        Ok(())
    }

    fn convert_frame(&mut self, image: RgbaImage, _frame_id: u32) -> Result<()> {
        if let Some(stdin) = &mut *self.stdin.borrow_mut() {
            stdin.write_all(&image.into_raw())?;
            Ok(())
        } else {
            Err(anyhow!("Encoder not prepared"))
        }
    }

    fn finish(&self) -> Result<()> {
        if let Some(mut stdin) = self.stdin.borrow_mut().take() {
            stdin.flush()?; // flush
            drop(stdin); 
        }

        if let Some(mut child) = self.encoder.borrow_mut().take() {
            let status = child.wait()?;
            if !status.success() {
                return Err(anyhow!("ffmpeg exited with status {}", status));
            }
        }

        Ok(())
    }
}
