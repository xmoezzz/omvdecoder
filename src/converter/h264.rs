use anyhow::Result;
use image::DynamicImage;
use minimp4::Mp4Muxer;
use std::{io::Cursor, path::{Path, PathBuf}};

use openh264::{
    encoder::Encoder,
    formats::{RgbaSliceU8, YUVBuffer},
};

use super::Converter;

pub struct H264Converter {
    path: PathBuf,
    encoder: Option<Encoder>,
    fps: f32,
    width: u32,
    height: u32,
    buffer: Vec<u8>,
}

impl H264Converter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            encoder: None,
            fps: 0.0,
            width: 0,
            height: 0,
            buffer: Vec::new(),
        }
    }
}

impl Converter for H264Converter {
    fn prepare(&mut self, width: u32, height: u32, fps: f32) -> Result<()> {
        self.fps = fps;
        self.width = width;
        self.height = height;
        self.encoder = Some(Encoder::new()?);
        Ok(())
    }

    fn convert_frame(&mut self, image: image::RgbaImage, _frame_id: u32) -> Result<()> {
        let image = DynamicImage::ImageRgba8(image);
        let image = RgbaSliceU8::new(
            image.as_bytes(),
            (self.width as usize, self.height as usize),
        );
        let yuv = YUVBuffer::from_rgb_source(image);
        match &mut self.encoder {
            Some(encoder) => {
                let bitstream = encoder.encode(&yuv)?;
                bitstream.write(&mut self.buffer)?;
            }
            None => {
                return Err(anyhow::anyhow!("Encoder not initialized"));
            }
        }

        Ok(())
    }

    fn finish(&self) -> Result<()> {

        // TODO:
        // bad design, will comsume a lot of memory
        let mut video_buffer = Cursor::new(Vec::new());
        let mut mp4muxer = Mp4Muxer::new(&mut video_buffer);
        mp4muxer.init_video(self.width as i32, self.height as i32, false, "");
        mp4muxer.write_video_with_fps(&self.buffer, self.fps as u32);
        mp4muxer.close();

        let path = self.path.with_extension("mp4");
        std::fs::write(path, video_buffer.into_inner())?;
        Ok(())
    }
}
