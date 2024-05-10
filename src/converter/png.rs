use anyhow::Result;
use std::path::{Path, PathBuf};

use super::Converter;

pub struct PngConverter {
    path: PathBuf,
}

impl PngConverter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Converter for PngConverter {
    fn prepare(&mut self, _width: u32, _height: u32, _fps: f32) -> Result<()> {
        if !self.path.exists() {
            std::fs::create_dir_all(&self.path)?;
        }
        Ok(())
    }

    fn convert_frame(&mut self, image: image::RgbaImage, frame_id: u32) -> Result<()> {
        let path = self.path.with_file_name(format!("frame_{:04}.png", frame_id));
        image.save(path)?;
        Ok(())
    }

    fn finish(&self) -> Result<()> {
        Ok(())
    }
}