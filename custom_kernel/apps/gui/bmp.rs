use alloc::vec::Vec;

#[derive(Clone)]
pub struct BmpImage {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u32>,
}

impl BmpImage {
    pub fn parse(raw_data: &[u8]) -> Option<Self> {
        if raw_data.len() < 54 {
            return None; // Too small to contain header
        }
        
        if raw_data[0] != b'B' || raw_data[1] != b'M' {
            return None; // Invalid magic
        }
        
        let data_offset = u32::from_le_bytes(raw_data[10..14].try_into().unwrap()) as usize;
        let width = i32::from_le_bytes(raw_data[18..22].try_into().unwrap()) as usize;
        let height = i32::from_le_bytes(raw_data[22..26].try_into().unwrap());
        let bpp = u16::from_le_bytes(raw_data[28..30].try_into().unwrap());
        
        if bpp != 32 && bpp != 24 {
            return None; // Only support 24 or 32 bpp for now
        }
        
        let abs_height = height.abs() as usize;
        let mut pixels = alloc::vec![0; width * abs_height];
        
        let row_stride = (width * (bpp as usize) / 8 + 3) & !3; // 4-byte aligned
        
        for y in 0..abs_height {
            let src_y = if height > 0 { abs_height - 1 - y } else { y }; // BMPs are usually bottom-up
            let row_offset = data_offset + src_y * row_stride;
            
            if row_offset + row_stride > raw_data.len() {
                break;
            }
            
            for x in 0..width {
                let px_offset = row_offset + x * (bpp as usize / 8);
                if bpp == 32 {
                    let b = raw_data[px_offset];
                    let g = raw_data[px_offset + 1];
                    let r = raw_data[px_offset + 2];
                    let a = raw_data[px_offset + 3]; // Usually unused or alpha
                    // Format as 0xAARRGGBB
                    pixels[y * width + x] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                } else if bpp == 24 {
                    let b = raw_data[px_offset];
                    let g = raw_data[px_offset + 1];
                    let r = raw_data[px_offset + 2];
                    pixels[y * width + x] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }
            }
        }
        
        Some(Self {
            width,
            height: abs_height,
            data: pixels,
        })
    }
}
