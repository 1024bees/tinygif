use core::ops::{Add, BitAnd, Shr};

use crate::common::{Block, ExtensionLabel, ParseError};
use crate::iterators::ByteIterator;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::iterators::SeekableIter;

use smallvec::SmallVec;
pub struct GifInfo {
    header: Header,
    control_info: Option<GraphicsControl>,
    pub(crate) image_block_locations: SmallVec<[usize; 128]>,
}

impl GifInfo {
    pub fn parser<S: SeekableIter>(raw_header: &mut ByteIterator<S>) -> Result<Self, ParseError> {
        let header = Header::parser(raw_header)?;
        let mut image_block_locations: SmallVec<[usize; 128]> = SmallVec::new();

        let mut control_info = None;
        loop {
            let block_id = raw_header.take_byte().map(|byte| Block::from_u8(byte))??;

            match block_id {
                Block::Image => {
                    image_block_locations.push(raw_header.get_offset());
                    //TODO:make this one call
                    let _ = LocalImageDescriptor::parser(raw_header)?;

                    skip_image_data(raw_header)?;
                }

                Block::Trailer => break,
                Block::Extension => {
                    let extension = raw_header
                        .take_byte()
                        .map(|byte| ExtensionLabel::from_u8(byte))??;
                    match extension {
                        ExtensionLabel::Graphics => {
                            control_info = Some(GraphicsControl::parse(raw_header)?);
                        }
                        _ => {
                            eat_extension(extension, raw_header)?;
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
    /// Delay between showing each gif frame, in miliseconds
    pub(crate) fn delay_time(&self) -> usize {
        self.control_info
            .as_ref()
            .map(|val| val.delay_time.clone() as usize)
            .unwrap_or(50)
    }
    pub(crate) fn num_images(&self) -> usize {
        self.image_block_locations.len()
    }

    pub(crate) fn base_size(&self) -> Size {
        self.header.image_size
    }
    pub(crate) fn global_table(&self) -> Option<&ColorTable> {
        self.header.global_table.as_ref().map(|val| &val.table)
    }
}

#[derive(Default)]
pub struct Header {
    /// Gif size in pixels.
    pub image_size: Size,

    /// Global color table,
    pub global_table: Option<GlobalColorTable>,
}

#[derive(Debug)]
pub struct LocalImageDescriptor {
    origin: Point,
    size: Size,
    interlaced: bool,
    local_color_table: Option<ColorTable>,
}

impl LocalImageDescriptor {
    ///Frame local [`ColorTable`], if it exists
    pub(crate) fn color_table(&self) -> Option<&ColorTable> {
        self.local_color_table.as_ref()
    }
    /// Total number of pixels in this frame
    pub(crate) fn num_pixels(&self) -> usize {
        (self.size.width * self.size.height) as usize
    }

    pub(crate) fn size(&self) -> Size {
        self.size
    }

    pub(crate) fn origin(&self) -> Point {
        self.origin
    }
    /// Area that the frame should be drawn to
    pub(crate) fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.origin, self.size)
    }
}

#[derive(Default)]
pub struct GlobalColorTable {
    background_color: u8,
    bits_per_pixel: u8,
    table: ColorTable,
}

#[derive(Debug)]
pub struct ColorTable {
    pub(crate) table: SmallVec<[Rgb565; 256]>,
}

/// Process for displaying next image in the file
///
enum DisposalMethod {
    NotSpecified = 0,
    DoNotDispose = 1,
    OverwriteWithBG = 2,
    OverwriteWithPrev = 3,
}

#[derive(Debug)]
pub struct GraphicsControl {
    /// Control byte
    ctrl: u8,
    ///table index for a transparent color
    transparent_idx: u8,
    ///Delay time, in second
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
    pub fn new<S: SeekableIter>(len: u16, iter: &mut ByteIterator<S>) -> Result<Self, ParseError> {
        let mut table = SmallVec::new();

        for _idx in 0..len {
            let r = iter.take_byte()?;
            let g = iter.take_byte()?;
            let b = iter.take_byte()?;
            table.push(Rgb565::from(Rgb888::new(r, g, b)))
        }

        Ok(Self { table })
    }
}

impl Header {
    pub fn parser<S: SeekableIter>(raw_header: &mut ByteIterator<S>) -> Result<Header, ParseError> {
        let name: [u8; 6] = raw_header.take_arr()?;

        if name.eq("GIF89a".as_bytes()) && name.eq("GIF87a".as_bytes()) {
            return Err(ParseError::BadGifFile);
        }

        let width = raw_header.take_u16_le()? as u32;
        let height = raw_header.take_u16_le()? as u32;

        let size = Size { width, height };

        let global_color_table_info = raw_header.take_byte()?;
        let global_color_exists = global_color_table_info.bitand(0x80).eq(&0x80);
        //FIXME: WHY DO WE NEED THIS? WE RANDOMLY SKIP TO THE 14th BYTE (IDX 13) IN THE ANIMATED GIF LIB??

        raw_header.take_byte()?;

        let global_table = if global_color_exists {
            let num_entries = 1 << (global_color_table_info.bitand(0x7).add(1));
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
    pub fn parser<S: SeekableIter>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<LocalImageDescriptor, ParseError> {
        let left = raw_header.take_u16_le()? as i32;
        let top = raw_header.take_u16_le()? as i32;
        let origin = Point { x: left, y: top };
        let width = raw_header.take_u16_le()? as u32;
        let height = raw_header.take_u16_le()? as u32;

        let size = Size { width, height };
        let local_color_table = Self::maybe_parse_local_color_table(raw_header)?;

        Ok(Self {
            origin,
            size,
            interlaced: false,
            local_color_table,
        })
    }
    /// Helper for
    fn maybe_parse_local_color_table<S: SeekableIter>(
        raw_header: &mut ByteIterator<S>,
    ) -> Result<Option<ColorTable>, ParseError> {
        let color_info = raw_header.take_byte()?;
        let interlaced = color_info.bitand(0x40).eq(&0x40);
        let has_local_table = color_info.bitand(0x80).eq(&0x80);
        let local_table = if has_local_table {
            let num_entries = 1 << (color_info.bitand(0x07).add(1));
            Some(ColorTable::new(num_entries, raw_header)?)
        } else {
            None
        };
        Ok(local_table)
    }
}

impl GraphicsControl {
    pub fn parse<S: SeekableIter>(raw_header: &mut ByteIterator<S>) -> Result<Self, ParseError> {
        let _len = raw_header.take_byte()?;
        //TODO: if len!= 4, throw error
        let ctrl = raw_header.take_byte()?;

        let delay_time = raw_header.take_u16_le()? * 10;

        let transparent_idx = raw_header.take_byte()?;

        // We gotta take the 0 value block delimiter still! Wahoo
        raw_header.take_byte()?;

        Ok(Self {
            delay_time,
            ctrl,
            transparent_idx,
        })
    }

    fn get_transparent_idx(&self) -> Option<u8> {
        self.ctrl.bitand(1).eq(&1).then_some(self.transparent_idx)
    }
    fn disposal_method(&self) -> u8 {
        self.ctrl.shr(2 as u8).bitand(0x2)
    }
}

fn eat_extension<S: SeekableIter>(
    _extension: ExtensionLabel,
    raw_header: &mut ByteIterator<S>,
) -> Result<(), ParseError> {
    while let len_byte = raw_header.take_byte()? {
        match len_byte {
            0 => return Ok(()),
            len_byte => raw_header.seek_by(len_byte as usize)?,
        }
    }
    Ok(())
}

/// Quick shim
fn skip_image_data<S: SeekableIter>(raw_header: &mut ByteIterator<S>) -> Result<(), ParseError> {
    let _code_size = raw_header.take_byte()?;
    while let val = raw_header.take_byte()? {
        match val {
            0 => break,
            _ => raw_header.seek_by(val as usize)?,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn sanity() {
        let crab = include_bytes!("test/crab.gif");
        let mut iter = ByteIterator::from_slice(crab);
        let gif_info = GifInfo::parser(&mut iter).unwrap();
        assert_eq!(gif_info.image_block_locations.len(), 60);
        assert_eq!(gif_info.control_info.as_ref().unwrap().delay_time, 90)
    }
}
