use std::io::{Cursor, Read};

use serde::Deserialize;

#[repr(C)]
#[derive(Copy, Clone, Debug, Deserialize)]
pub struct SprHeader {
    pub id: [u8; 4],
    pub version: i32,
    pub sprite_ty: i32,
    pub text_format: i32,
    pub bounding_radius: f32,
    pub max_width: i32,
    pub max_height: i32,
    pub frame_num: i32,
    pub beam_length: f32,
    pub sync_type: i32,
    pub palette_color_count: u16,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Deserialize)]
pub struct SprFrameHeader {
    pub group: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SprFile {
    pub header: SprHeader,
    pub palette: Vec<[u8; 3]>,
    pub frames: Vec<SprFrame>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SprFrame {
    pub header: SprFrameHeader,
    pub data: Vec<u8>,
}

impl SprFile {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut reader = Cursor::new(bytes);
        let header: SprHeader = bincode::deserialize_from(&mut reader).unwrap();
        let num_colors = header.palette_color_count as usize;
        let mut palette_data = vec![0u8; num_colors * 3];
        reader.read_exact(&mut palette_data).unwrap();
        let palette: Vec<[u8; 3]> = palette_data
            .chunks_exact(3)
            .map(|x| x.try_into().unwrap())
            .collect();
        let mut frames = Vec::with_capacity(header.frame_num as usize);
        for _ in 0..header.frame_num {
            let frame_header: SprFrameHeader = bincode::deserialize_from(&mut reader).unwrap();
            let mut data = vec![0u8; frame_header.width as usize * frame_header.height as usize];
            reader.read_exact(&mut data).unwrap();
            frames.push(SprFrame {
                header: frame_header,
                data,
            });
        }
        Self {
            header,
            palette,
            frames,
        }
    }

    pub fn decode_frame(&self, frame_index: usize) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let frame = &self.frames[frame_index];

        // https://developer.valvesoftware.com/wiki/SPR
        let transparent_index = if self.header.text_format == 3 {
            Some(self.header.palette_color_count as usize - 1)
        } else {
            None
        };

        let mut new_pixels =
            Vec::with_capacity(frame.header.width as usize * frame.header.height as usize * 4);
        for pixel in &frame.data {
            let pixel = *pixel as usize;
            let is_transparent = transparent_index.map(|x| x == pixel).unwrap_or(false);
            if !is_transparent {
                let color = self.palette[pixel];
                new_pixels.push(color[0]);
                new_pixels.push(color[1]);
                new_pixels.push(color[2]);
                new_pixels.push(255);
            } else {
                new_pixels.push(0);
                new_pixels.push(0);
                new_pixels.push(0);
                new_pixels.push(0);
            }
        }
        image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
            frame.header.width as u32,
            frame.header.height as u32,
            new_pixels,
        )
        .unwrap()
    }
}
