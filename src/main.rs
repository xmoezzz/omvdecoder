use anyhow::Result;
use binrw::BinRead;
use clap::Parser;
use kmpsearch::Haystack;
use memmap::MmapOptions;
use pack::OmvHeader;
use serde::{Deserialize, Serialize};
use std::alloc::Layout;
use std::fs::File;
use std::path::{Path, PathBuf};
use theorafile_rs::*;

use crate::converter::Converter;

mod converter;
mod pack;

fn read_omv_header(source: &[u8]) -> Result<pack::OmvHeader> {
    let mut source = std::io::Cursor::new(source);
    let header = pack::OmvHeader::read(&mut source)?;
    Ok(header)
}

fn convert_file(
    path: impl AsRef<Path>,
    output_format: OutputFormat,
    output: impl AsRef<Path>,
) -> Result<()> {
    let file = File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let mem = mmap.as_ref();
    let header = read_omv_header(mem)?;
    log::info!("extracting {:?}", header);
    let res = mem
        .indexesof_needle(b"OggS")
        .ok_or(anyhow::anyhow!("OggS not found"))?;
    let first_index = res
        .first()
        .ok_or(anyhow::anyhow!("OggS not found"))?
        .to_owned();
    let ogv_content = &mem[first_index..];

    convert_embedded_ogv(&header, ogv_content, output_format, output)?;

    Ok(())
}

pub struct DataSource {
    data: Vec<u8>,
    pos: usize,
}

impl DataSource {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    // Implement a method to seek to a specific position
    pub fn seek(
        &mut self,
        offset: ogg_int64_t,
        origin: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int {
        match origin {
            0 => self.pos = offset as usize,
            1 => self.pos = (self.pos as ogg_int64_t + offset) as usize,
            2 => self.pos = (self.data.len() as ogg_int64_t + offset) as usize,
            _ => return -1, // Unsupported origin
        }
        // Ensure pos doesn't go out of bounds
        if self.pos > self.data.len() {
            self.pos = self.data.len();
        }
        0 // Success
    }

    // Implement a method to read data from the current position
    pub fn read(&mut self, ptr: *mut ::std::os::raw::c_void, size: usize, nmemb: usize) -> usize {
        let bytes_to_read = size * nmemb;
        let remaining_data = &self.data[self.pos..];
        let bytes_read = std::cmp::min(remaining_data.len(), bytes_to_read);
        unsafe {
            std::ptr::copy_nonoverlapping(remaining_data.as_ptr(), ptr as *mut u8, bytes_read);
        }
        self.pos += bytes_read;
        bytes_read
    }

    // Implement a method to close the data source
    pub fn close(&mut self) -> ::std::os::raw::c_int {
        // Optionally perform any cleanup here
        0 // Success
    }
}

unsafe extern "C" fn read_func_impl(
    ptr: *mut ::std::os::raw::c_void,
    size: usize,
    nmemb: usize,
    datasource: *mut ::std::os::raw::c_void,
) -> usize {
    if let Some(datasource) = (datasource as *mut DataSource).as_mut() {
        datasource.read(ptr, size, nmemb)
    } else {
        0
    }
}

unsafe extern "C" fn seek_func_impl(
    datasource: *mut ::std::os::raw::c_void,
    offset: ogg_int64_t,
    origin: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    if let Some(datasource) = (datasource as *mut DataSource).as_mut() {
        datasource.seek(offset, origin)
    } else {
        -1
    }
}

unsafe extern "C" fn close_func_impl(
    datasource: *mut ::std::os::raw::c_void,
) -> ::std::os::raw::c_int {
    if let Some(datasource) = (datasource as *mut DataSource).as_mut() {
        datasource.close()
    } else {
        -1
    }
}

#[inline]
fn clamp(val: f32) -> u8 {
    if val < 0.0 {
        0
    } else if val > 255.0 {
        255
    } else {
        val.round() as u8
    }
}

fn yuv2rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let r = y as f32 + (1.370705 * (v as f32 - 128.0));
    let g = y as f32 - (0.698001 * (v as f32 - 128.0)) - (0.337633 * (u as f32 - 128.0));
    let b = y as f32 + (1.732446 * (u as f32 - 128.0));
    let r = clamp(r);
    let g = clamp(g);
    let b = clamp(b);
    return (r, g, b);
}

fn yuv_to_image(data: *mut i8, width: i32, height: i32, is24bit: bool) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(width as u32, height as u32);

    // for (int y = 0; y < height; y++)
    // {
    //     for (int x = 0; x < width; x++)
    //     {
    //         buf[offset + data.Stride * y + 4 * x + 0] = vidbuf[vidwidth * (vidheight * 0 + y) + x];
    //         buf[offset + data.Stride * y + 4 * x + 1] = vidbuf[vidwidth * (vidheight * 1 + y) + x];
    //         buf[offset + data.Stride * y + 4 * x + 2] = vidbuf[vidwidth * (vidheight * 2 + y) + x];
    //         if (y < (height + 2) / 3)
    //         {
    //             buf[offset + data.Stride * y + 4 * x + 3] = vidbuf[vidwidth * (height * 1 + y) + x];
    //         }
    //         else if (y < (height + 2) / 3 * 2)
    //         {
    //             buf[offset + data.Stride * y + 4 * x + 3] = vidbuf[vidwidth * (height * 2 + y) + x];
    //         }
    //         else
    //         {
    //             buf[offset + data.Stride * y + 4 * x + 3] = vidbuf[vidwidth * (height * 3 + y) + x];
    //         }
    //     }
    // }

    #[allow(clippy::erasing_op)]
    #[allow(clippy::identity_op)]
    for y in 0..height {
        for x in 0..width {
            let b = unsafe { *data.offset((width * (height * 0 + y) + x) as isize) };
            let g = unsafe { *data.offset((width * (height * 1 + y) + x) as isize) };
            let r = unsafe { *data.offset((width * (height * 2 + y) + x) as isize) };

            let a = if is24bit {
                0xff
            } else {
                let a = if y < (height + 2) / 3 {
                    unsafe { *data.offset((x + width * (height * 1 + y)) as isize) }
                } else if y < (height + 2) / 3 * 2 {
                    unsafe { *data.offset((x + width * (height * 2 + y)) as isize) }
                } else {
                    unsafe { *data.offset((x + width * (height * 3 + y)) as isize) }
                };
                a as u8
            };

            img.put_pixel(
                x as u32,
                y as u32,
                image::Rgba([r as u8, g as u8, b as u8, a]),
            );
        }
    }
    img
}

fn convert_embedded_ogv(
    header: &OmvHeader,
    ogv_content: &[u8],
    output_format: OutputFormat,
    output: impl AsRef<Path>,
) -> Result<()> {
    let mut tf_cbs = tf_callbacks {
        read_func: Some(read_func_impl),
        seek_func: Some(seek_func_impl),
        close_func: Some(close_func_impl),
    };

    let mut converter: Box<dyn Converter> = match output_format {
        OutputFormat::H264 => {
            log::info!("Converting to H264");
            let cvt = converter::H264Converter::new(output);
            Box::new(cvt)
        }
        OutputFormat::PngPicture => {
            log::info!("Converting to PNG");
            let cvt = converter::PngConverter::new(output);
            Box::new(cvt)
        }
        OutputFormat::JpgPicture => {
            log::info!("Converting to JPG");
            let cvt = converter::JpgConverter::new(output);
            Box::new(cvt)
        }
        OutputFormat::Ffmpeg => {
            log::info!("Converting using Ffmpeg");
            let cvt = converter::FfmepgConverter::new(output);
            Box::new(cvt)
        }
    };

    let datasource = DataSource::new(ogv_content.to_vec());
    let datasource_ptr =
        &datasource as *const DataSource as *mut DataSource as *mut ::std::os::raw::c_void;

    let layout = Layout::new::<OggTheora_File>();
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        return Err(anyhow::anyhow!("Failed to allocate memory"));
    }

    let ogg_file = ptr as *mut OggTheora_File;
    let ret = unsafe { tf_open_callbacks(datasource_ptr, ogg_file, tf_cbs) };
    if ret < 0 {
        if !ptr.is_null() {
            unsafe { std::alloc::dealloc(ptr, layout) };
        }
        return Err(anyhow::anyhow!("Failed to open OggTheora file"));
    }

    let ret = unsafe { tf_hasvideo(ogg_file) };
    if ret == 0 {
        unsafe { tf_close(ogg_file) };
        if !ptr.is_null() {
            unsafe { std::alloc::dealloc(ptr, layout) };
        }
        return Err(anyhow::anyhow!("No video stream found"));
    }

    let mut width: ::std::os::raw::c_int = 0;
    let mut height: ::std::os::raw::c_int = 0;
    let mut fps: f64 = 0.0;
    let mut fmt: th_pixel_fmt = 0;
    let mut frame_count = 0;

    unsafe { tf_videoinfo(ogg_file, &mut width, &mut height, &mut fps, &mut fmt) };

    if fmt != th_pixel_fmt_TH_PF_444 {
        unsafe { tf_close(ogg_file) };
        if !ptr.is_null() {
            unsafe { std::alloc::dealloc(ptr, layout) };
        }
        return Err(anyhow::anyhow!("Unsupported pixel format"));
    }

    converter.prepare(width as u32, height as u32, fps as f32)?;

    let size = width as usize * height as usize * 3;
    let alignment = 1024;
    let mem_layout = unsafe { Layout::from_size_align_unchecked(size, alignment) };

    let data_blob = unsafe { std::alloc::alloc(mem_layout) as *mut i8 };

    log::info!(
        "width: {}, height: {}, fps: {}, fmt: {}",
        width,
        height,
        fps,
        fmt
    );

    if data_blob.is_null() {
        unsafe { tf_close(ogg_file) };
        if !ptr.is_null() {
            unsafe { std::alloc::dealloc(ptr, layout) };
        }
        return Err(anyhow::anyhow!("Failed to allocate memory for video data"));
    }

    loop {
        if unsafe { tf_eos(ogg_file) } != 0 {
            break;
        }

        unsafe { tf_readvideo(ogg_file, data_blob, 1) };
        let image = yuv_to_image(
            data_blob,
            width,
            height,
            header.metadata.height == height as u32,
        );
        converter.convert_frame(image, frame_count)?;
        frame_count += 1;
        log::info!("Decoded {} frame(s)", frame_count);
    }

    unsafe { tf_close(ogg_file) };
    if !ptr.is_null() {
        unsafe { std::alloc::dealloc(ptr, layout) };
    }
    if !data_blob.is_null() {
        unsafe { std::alloc::dealloc(data_blob as *mut u8, mem_layout) };
    }

    converter.finish()?;

    Ok(())
}

#[derive(Debug, clap::ValueEnum, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum OutputFormat {
    H264,
    PngPicture,
    #[default]
    JpgPicture,
    Ffmpeg,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    output: PathBuf,

    #[arg(short, long)]
    format: OutputFormat,
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let args = Args::parse();
    convert_file(args.input, args.format, args.output).unwrap();
}
