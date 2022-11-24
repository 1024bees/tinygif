use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, Size},
    primitives::Rectangle,
};
use weezl::{decode::Decoder, BitOrder, LzwStatus};

use crate::{
    common::ParseError,
    header::{ColorTable, GifInfo, LocalImageDescriptor},
    iterators::{ByteIterator, SeekableIter, SeekableSliceIter},
};

pub struct GifFrameStreamer<S: SeekableIter> {
    pub(crate) header_info: GifInfo,
    frame_offset: usize,
    local_image_descriptor: Option<LocalImageDescriptor>,
    bytes: ByteIterator<S>,
}

impl<S: SeekableIter> GifFrameStreamer<S> {
    pub fn num_images(&self) -> usize {
        self.header_info.num_images()
    }

    /// Amount of time to delay until the next frame
    pub fn delay_ms(&self) -> usize {
        self.header_info.delay_time()
    }

    pub fn base_size(&self) -> Size {
        self.header_info.base_size()
    }

    pub fn reset(&mut self) -> Result<(), ParseError> {
        self.bytes.seek_to(0)?;
        self.frame_offset = 0;
        Ok(())
    }
}

struct LilQ {
    buf: [u8; 255],
    idx: usize,
    size: usize,
}

impl LilQ {
    fn new() -> Self {
        Self {
            buf: [0; 255],
            idx: 0,
            size: 0,
        }
    }

    #[inline]
    fn empty(&self) -> bool {
        self.idx >= self.size
    }

    fn next(&mut self) -> Option<u8> {
        (!self.empty()).then(|| {
            let rv: u8 = self.buf[self.idx as usize];
            self.idx += 1;
            rv
        })
    }

    fn live_slice(&mut self) -> &mut [u8] {
        &mut self.buf[(self.idx as usize)..(self.size as usize)]
    }

    fn reset(&mut self) {
        self.idx = 0;
        self.size = 0;
    }
}

impl<'iter> GifFrameStreamer<SeekableSliceIter<'iter>> {
    pub fn from_slice(
        slice: &'iter [u8],
    ) -> Result<GifFrameStreamer<SeekableSliceIter<'iter>>, ParseError> {
        let mut bytes = ByteIterator::from_slice(slice);
        let header_info = GifInfo::parser(&mut bytes)?;
        bytes.seek_to(0)?;

        Ok(Self {
            bytes,
            frame_offset: 0,
            local_image_descriptor: None,
            header_info,
        })
    }
}

impl<S: SeekableIter> GifFrameStreamer<S> {
    pub fn new(header_info: GifInfo, bytes: ByteIterator<S>) -> Self {
        Self {
            bytes,
            frame_offset: 0,
            local_image_descriptor: None,
            header_info,
        }
    }

    pub fn seek_to_next_frame(&mut self) -> Result<(), ParseError> {
        let offset = self
            .header_info
            .image_block_locations
            .get(self.frame_offset)
            .ok_or(ParseError::NoImagesLeft)
            .cloned();

        let offset = match offset {
            Ok(offset) => offset,
            Err(_) => {
                self.frame_offset = 0;
                self.header_info
                    .image_block_locations
                    .get(self.frame_offset)
                    .ok_or(ParseError::BadGifFile)?
                    .clone()
            }
        };

        self.frame_offset += 1;

        self.bytes.seek_to(offset)?;
        self.local_image_descriptor = Some(LocalImageDescriptor::parser(&mut self.bytes)?);
        Ok(())
    }

    pub fn current_frame(&self) -> Result<GifFrame<'_, S>, ParseError> {
        let color_table = self
            .local_image_descriptor
            .as_ref()
            .unwrap()
            .color_table()
            .unwrap_or_else(|| self.header_info.global_table().unwrap());

        Ok(GifFrame::new(
            self.bytes.clone(),
            color_table,
            self.local_image_descriptor.as_ref().unwrap(),
        ))
    }

    pub fn next_frame(&mut self) -> Result<GifFrame<'_, S>, ParseError> {
        self.seek_to_next_frame()?;
        let color_table = self
            .local_image_descriptor
            .as_ref()
            .unwrap()
            .color_table()
            .unwrap_or_else(|| self.header_info.global_table().unwrap());

        Ok(GifFrame::new(
            self.bytes.clone(),
            color_table,
            self.local_image_descriptor.as_ref().unwrap(),
        ))
    }
}

pub(crate) enum DecodeState {
    /// processing a sub-block hasnt started
    NewSubBlock,
    /// Processing a sublock
    ProcessingSubBlock,
    /// Block is done
    BlockDone,
    /// Gif frame is fully processed
    FrameDone,
}

pub struct GifFrame<'header, S: SeekableIter> {
    bytes: ByteIterator<S>,
    color_table: &'header ColorTable,
    image_descriptor: &'header LocalImageDescriptor,
    decoder: Decoder,
    /// Buffer that we write sub-blocks into
    block_buffer: LilQ,
    /// Buffer that we decode the LZW stream into
    decode_buffer: LilQ,
    pub(crate) state: DecodeState,
}

pub struct GifFramePixelIter<'header, S: SeekableIter> {
    bytes: ByteIterator<S>,
    color_table: &'header ColorTable,
    image_descriptor: &'header LocalImageDescriptor,
    decoder: Decoder,
    /// Buffer that we write sub-blocks into
    block_buffer: LilQ,
    /// Buffer that we decode the LZW stream into
    decode_buffer: LilQ,
    pub(crate) state: DecodeState,
}

impl<'header, S> GifFrame<'header, S>
where
    S: SeekableIter,
{
    pub fn new(
        mut bytes: ByteIterator<S>,
        color_table: &'header ColorTable,
        image_descriptor: &'header LocalImageDescriptor,
    ) -> Self {
        let code_size = bytes.take_byte().unwrap();
        Self {
            bytes,
            color_table,
            image_descriptor,
            decoder: Decoder::new(BitOrder::Lsb, code_size),
            decode_buffer: LilQ::new(),
            block_buffer: LilQ::new(),
            state: DecodeState::NewSubBlock,
        }
    }

    pub fn done(&self) -> bool {
        match self.state {
            DecodeState::FrameDone => true,
            _ => false,
        }
    }

    pub fn frame_area(&self) -> Rectangle {
        self.image_descriptor.bounding_box()
    }

    pub fn img_size(&self) -> Size {
        self.image_descriptor.size()
    }

    pub fn origin(&self) -> Point {
        self.image_descriptor.origin()
    }

    fn fill_block_buffer(&mut self) -> Result<(), ParseError> {
        self.block_buffer.reset();
        let num_bytes = self.bytes.take_byte()?;
        if num_bytes == 0 {
            self.state = DecodeState::FrameDone;
        } else {
            self.state = DecodeState::NewSubBlock;
            for idx in 0..num_bytes {
                self.block_buffer.buf[idx as usize] = self.bytes.take_byte()?;
            }
        }
        self.block_buffer.size = num_bytes as usize;
        Ok(())
    }

    fn fill_decode_buffer(&mut self) {
        if !self.done() {
            self.decode_buffer.reset();
            while self.decode_buffer.empty() {
                match self.state {
                    DecodeState::NewSubBlock => {
                        self.fill_block_buffer().unwrap();
                    }
                    DecodeState::BlockDone => {
                        self.fill_block_buffer().unwrap();
                        if self.done() {
                            return;
                        }
                    }

                    _ => {}
                }

                let out = self.decoder.decode_bytes(
                    self.block_buffer.live_slice(),
                    &mut self.decode_buffer.buf[..],
                );

                let (consumed_in, consumed_out) = (out.consumed_in, out.consumed_out);
                match out.status.unwrap() {
                    LzwStatus::NoProgress | LzwStatus::Done => self.state = DecodeState::BlockDone,
                    LzwStatus::Ok => {
                        if consumed_in == 0 && consumed_out == 0 {
                            panic!("ooooooh noo");
                        }
                        self.state = DecodeState::ProcessingSubBlock;
                    }
                }

                self.decode_buffer.size = consumed_out;
                self.block_buffer.idx += consumed_in;
            }
        }
    }

    pub fn num_pixels(&self) -> usize {
        self.image_descriptor.num_pixels()
    }
}

impl<S: SeekableIter> Iterator for GifFrame<'_, S> {
    ///TODO: Suppport other colors
    type Item = Rgb565;
    fn next(&mut self) -> Option<Self::Item> {
        if self.decode_buffer.empty() {
            self.fill_decode_buffer()
        }
        let val = self.decode_buffer.next();
        val.map(|idx| self.color_table.table[idx as usize])
    }
}

#[cfg(test)]
mod tests {

    use embedded_graphics::{image::Image, prelude::*};

    use super::*;
    use crate::test_utils::Framebuffer;
    use std::{io::Cursor, vec::Vec};

    fn iterate_gif(bytes: &[u8]) {
        let mut gif = GifFrameStreamer::from_slice(bytes).unwrap();
        for _ in 0..gif.header_info.num_images() {
            let frame = gif.next_frame().unwrap();
            let num_pixels = frame.image_descriptor.num_pixels();
            let pixels: Vec<Rgb565> = frame.into_iter().collect();
            assert_eq!(pixels.len(), num_pixels);
        }
    }

    #[test]
    fn crab_frames() {
        let crab = include_bytes!("test/crab.gif");
        let mut iter = ByteIterator::from_slice(crab);
        let gif_info = GifInfo::parser(&mut iter).unwrap();
        let iter2 = ByteIterator::from_slice(crab);
        let mut frames = GifFrameStreamer::new(gif_info, iter2);
        let num_frames = frames.header_info.num_images();

        for _ in 0..num_frames {
            let frame = frames.next_frame().unwrap();
            let num_pixels = frame.image_descriptor.num_pixels();
            let pixels: Vec<Rgb565> = frame.into_iter().collect();
            assert_eq!(pixels.len(), num_pixels);
        }
    }

    #[test]
    fn api_crab() {
        let crab = include_bytes!("test/crab.gif");
        iterate_gif(crab);
    }

    #[test]
    fn api_bee() {
        let bee = include_bytes!("test/bee.gif");
        iterate_gif(bee);
    }

    #[test]
    fn api_bee_framebuffer() {
        let bee = include_bytes!("test/bee.gif");
        let mut gif = GifFrameStreamer::from_slice(bee).unwrap();

        let mut fb = Framebuffer::<240, 240>::new();
        for _ in 0..100 {
            gif.seek_to_next_frame().unwrap();
            Image::new(&gif, Point::zero()).draw(&mut fb).unwrap();
        }
    }
}
