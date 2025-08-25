use anyhow::{anyhow, Result};
use image::{ImageEncoder, RgbaImage};
use image::codecs::png::PngEncoder;
use std::io::{Write};
use std::path::Path;
use image::ExtendedColorType;

use crate::Converter;

pub struct PipedPngConverter {
    fps: f32,
    width: u32,
    height: u32,
    header_written: bool,
}

impl PipedPngConverter {
    pub fn new(_dummy_out: impl AsRef<Path>) -> Self {
        Self {
            fps: 0.0,
            width: 0,
            height: 0,
            header_written: false,
        }
    }

    #[inline]
    fn write_header_if_needed(&mut self) -> Result<()> {
        if self.header_written {
            return Ok(());
        }
        let (num, den) = fps_to_rational(self.fps);
        let mut out = std::io::stdout().lock();
        // container header（y4m-like）
        // PNG payload
        writeln!(out, "PXY4M W{} H{} F{}/{} Crgba Enc:png", self.width, self.height, num, den)?;
        out.flush()?;
        self.header_written = true;
        Ok(())
    }
}


impl Converter for PipedPngConverter {
    fn prepare(&mut self, width: u32, height: u32, fps: f32) -> Result<()> {
        self.width = width;
        self.height = height;
        self.fps = fps;
        self.write_header_if_needed()
    }

    fn convert_frame(&mut self, image: RgbaImage, frame_id: u32) -> Result<()> {
        if image.width() != self.width || image.height() != self.height {
            return Err(anyhow!(
                "frame size mismatch: expected {}x{}, got {}x{}",
                self.width, self.height, image.width(), image.height()
            ));
        }

        let png_buf = Vec::with_capacity((self.width * self.height * 4) as usize);
        // image to png
        let mut png_buf = png_buf;
        let encoder = PngEncoder::new(&mut png_buf);
        encoder.write_image(&image.into_raw(), self.width, self.height, ExtendedColorType::Rgba8)?;

        // write the frame header + PNG
        let mut out = std::io::stdout().lock();
        writeln!(out, "FRAME")?;
        writeln!(out, "PTS {}", frame_id)?;
        writeln!(out, "BYTES {}", png_buf.len())?;
        out.write_all(&png_buf)?;
        out.flush()?; 

        Ok(())
    }

    fn finish(&self) -> Result<()> {
        let mut out = std::io::stdout();
        out.flush()?;
        Ok(())
    }
}


fn fps_to_rational(fps: f32) -> (u32, u32) {
    if (fps.fract()).abs() < 1e-6 {
        return (fps.round() as u32, 1);
    }
    let den = 1000u32;
    let num = (fps * den as f32).round() as u32;
    let g = gcd_u32(num, den);
    (num / g, den / g)
}

fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}
