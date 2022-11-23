use core::{
    mem::size_of,
    ops::{Add, BitAnd, Shr},
    ptr::null,
};

use crate::common::{Block, ExtensionLabel, ParseError};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use smallvec::{smallvec, SmallVec};
pub struct GifInfo {
    header: Header,
    control_info: Option<GraphicsControl>,
    image_block_locations: SmallVec<[usize; 128]>,
}

impl GifInfo {
    pub fn parser<S: Iterator<Item = u8>>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<Self, ParseError> {
        let header = Header::parser(raw_header)?;
        let mut image_block_locations: SmallVec<[usize; 128]> = SmallVec::new();

        let mut control_info = None;
        loop {
            match raw_header.take_byte().map(|byte| Block::from_u8(byte))?? {
                Block::Image => {
                    println!("Found an image at {}", raw_header.get_offset());
                    image_block_locations.push(raw_header.get_offset());
                    //TODO:make this one call
                    LocalImageDescriptor::parser(raw_header)?;
                    LocalImageDescriptor::maybe_parse_local_color_table(raw_header)?;
                    skip_image_data(raw_header)?;
                }

                Block::Trailer => break,
                Block::Extension => {
                    let extension = raw_header
                        .take_byte()
                        .map(|byte| ExtensionLabel::from_u8(byte))??;
                    match extension {
                        ExtensionLabel::Graphics => {
                            println!("Found a graphics extension at {}", raw_header.get_offset());

                            control_info = Some(GraphicsControl::parse(raw_header)?);
                        }
                        _ => {
                            println!(
                                "Ate some useless extension {:?} at {}",
                                extension,
                                raw_header.get_offset()
                            );

                            eat_extension(raw_header)?;
                        }
                    }
                }
            }
        }

        Ok(Self {
            header,
            image_block_locations,
            control_info,
        })
    }
}

#[derive(Default)]
pub struct Header {
    /// Gif size in pixels.
    pub image_size: Size,

    /// Global color table (if it exists)
    pub global_table: Option<GlobalColorTable>,
}

pub struct LocalImageDescriptor {
    origin: Point,
    size: Size,
    interlaced: bool,
    color_table: *const ColorTable,
}

#[derive(Default)]
pub struct GlobalColorTable {
    background_color: u8,
    bits_per_pixel: u8,
    table: ColorTable,
}

pub struct ColorTable {
    table: SmallVec<[Rgb565; 256]>,
}

/// Process for displaying next image in the file
///
enum DisposalMethod {
    NotSpecified = 0,
    DoNotDispose = 1,
    OverwriteWithBG = 2,
    OverwriteWithPrev = 3,
}

pub struct GraphicsControl {
    /// Control byte
    ctrl: u8,
    ///table index for a transparent color
    transparent_idx: u8,
    ///Delay time, in hundredths of a second
    delay_time: u16,
}

impl Default for ColorTable {
    fn default() -> Self {
        Self {
            table: SmallVec::new(),
        }
    }
}

impl ColorTable {
    pub fn new<S: Iterator<Item = u8>>(
        len: u16,
        iter: &mut ByteIterator<S>,
    ) -> Result<Self, ParseError> {
        let mut table = SmallVec::new();

        for idx in 0..len {
            let r = iter.take_byte()?;
            let g = iter.take_byte()?;
            let b = iter.take_byte()?;
            table.push(Rgb565::new(r, g, b))
        }
        println!("Done with Color Table init!");
        Ok(Self { table })
    }
}

pub struct ByteIterator<S: Iterator<Item = u8>>(S, usize);

impl<S: Iterator<Item = u8>> ByteIterator<S> {
    fn take_u16_le(&mut self) -> Result<u16, ParseError> {
        self.1 += size_of::<u16>();
        self.0
            .next_chunk()
            .map(|val| u16::from_le_bytes(val))
            .map_err(|_| ParseError::UnepectedEOF)
    }

    fn take_byte(&mut self) -> Result<u8, ParseError> {
        self.1 += size_of::<u8>();
        let res = self.0.next().ok_or(ParseError::UnepectedEOF);

        res
    }

    #[inline]
    fn take_arr<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        self.1 += size_of::<[u8; N]>();
        self.0.next_chunk().map_err(|_| ParseError::UnepectedEOF)
    }
    fn get_offset(&self) -> usize {
        self.1
    }

    fn seek_by(&mut self, len: usize) -> Result<(), ParseError> {
        self.1 += len;
        self.0.advance_by(len).map_err(|_| ParseError::UnepectedEOF)
    }
}

impl Header {
    pub fn parser<S: Iterator<Item = u8>>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<Header, ParseError> {
        let name: [u8; 6] = raw_header.take_arr()?;

        if (name.eq("GIF89a".as_bytes()) && name.eq("GIF87a".as_bytes())) {
            return Err(ParseError::BadGifFile);
        }

        let width = raw_header.take_u16_le()? as u32;
        let height = raw_header.take_u16_le()? as u32;

        let size = Size { width, height };

        let global_color_table_info = raw_header.take_byte()?;
        let is_present = global_color_table_info.bitand(0x80).eq(&0x80);
        //FIXME: WHY DO WE NEED THIS? WE RANDOMLY SKIP TO THE 14th BYTE (IDX 13) IN THE ANIMATED GIF LIB??

        raw_header.take_byte()?;

        let global_table = if is_present {
            let num_entries = (1 << (global_color_table_info.bitand(0x7).add(1)));
            let bits_per_pixel = global_color_table_info.bitand(0x70).shr(4) + 1 as u8;
            let background_color = raw_header.take_byte()?;
            let table = ColorTable::new(num_entries, raw_header)?;
            Some(GlobalColorTable {
                background_color,
                bits_per_pixel,
                table,
            })
        } else {
            None
        };
        Ok(Header {
            image_size: size,
            global_table,
        })
    }
}

impl LocalImageDescriptor {
    pub fn parser<S: Iterator<Item = u8>>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<LocalImageDescriptor, ParseError> {
        let left = raw_header.take_u16_le()? as i32;
        let top = raw_header.take_u16_le()? as i32;
        let origin = Point { x: left, y: top };
        let width = raw_header.take_u16_le()? as u32;
        let height = raw_header.take_u16_le()? as u32;

        let size = Size { width, height };

        Ok(Self {
            origin,
            size,
            interlaced: false,
            color_table: null(),
        })
    }
    pub fn maybe_parse_local_color_table<S: Iterator<Item = u8>>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<Option<ColorTable>, ParseError> {
        let color_info = raw_header.take_byte()?;
        let interlaced = color_info.bitand(0x2).eq(&0x2);
        let has_local_table = color_info.bitand(0x1).eq(&0x1);
        let local_table = if has_local_table {
            let num_entries = 1 << (color_info.bitand(0x70).shr(4 as u8).add(1));
            Some(ColorTable::new(num_entries, raw_header)?)
        } else {
            None
        };
        Ok(local_table)
    }
}

impl GraphicsControl {
    pub fn parse<S: Iterator<Item = u8>>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<Self, ParseError> {
        let len = raw_header.take_byte()?;
        //TODO: if len!= 4, throw error
        let ctrl = raw_header.take_byte()?;

        let delay_time = raw_header.take_u16_le()? * 10;

        let transparent_idx = raw_header.take_byte()?;

        Ok(Self {
            delay_time,
            ctrl,
            transparent_idx,
        })
    }

    pub fn get_transparent_idx(&self) -> Option<u8> {
        self.ctrl.bitand(1).eq(&1).then_some(self.transparent_idx)
    }
    pub fn disposal_method(&self) -> u8 {
        self.ctrl.shr(2 as u8).bitand(0x2)
    }
}

pub fn eat_extension<S: Iterator<Item = u8>>(
    raw_header: &mut ByteIterator<S>,
) -> Result<(), ParseError> {
    while let len_byte = raw_header.take_byte()? {
        raw_header.seek_by(len_byte as usize)?;
    }
    let terminator = raw_header.take_byte()?;
    assert_eq!(terminator, 0);
    Ok(())
}

pub fn skip_image_data<S: Iterator<Item = u8>>(
    raw_header: &mut ByteIterator<S>,
) -> Result<(), ParseError> {
    while let val = raw_header.take_byte()? {
        match val {
            0 => return Ok(()),
            _ => raw_header.seek_by(val as usize)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn sanity() {
        let crab = include_bytes!("test/crab.gif");
        let golden = gif::DecodeOptions::new();
        let cursor = Cursor::new(crab);
        let dec2 = golden.read_info(cursor).unwrap();
        let pal = dec2.global_palette().unwrap();
        let mut iter = ByteIterator(crab.into_iter().cloned(), 0);
        let gif_info = GifInfo::parser(&mut iter).unwrap();
    }
}
