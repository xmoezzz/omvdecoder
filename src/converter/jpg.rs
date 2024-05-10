use anyhow::Result;
use std::path::{Path, PathBuf};

use super::Converter;

pub struct JpgConverter {
    path: PathBuf,
}

impl JpgConverter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Converter for JpgConverter {
    fn prepare(&mut self, _width: u32, _height: u32, _fps: f32) -> Result<()> {
        if !self.path.exists() {
            std::fs::create_dir_all(&self.path)?;
        }
        Ok(())
    }

    fn convert_frame(&mut self, image: image::RgbaImage, frame_id: u32) -> Result<()> {
        let path = self.path.with_file_name(format!("frame_{:04}.jpg", frame_id));
        image.save(path)?;
        Ok(())
    }

    fn finish(&self) -> Result<()> {
        Ok(())
    }
}