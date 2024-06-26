use anyhow::Result;

mod png;
mod jpg;
mod h264;

pub use {png::PngConverter, jpg::JpgConverter, h264::H264Converter};

pub trait Converter {
    fn prepare(&mut self, width: u32, height: u32, fps: f32) -> Result<()>;
    fn convert_frame(&mut self, image: image::RgbaImage, frame_id: u32) -> Result<()>;
    fn finish(&self) -> Result<()>;
}
