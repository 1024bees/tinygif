//! A small and opinionated gif decoder, primarily for embedded, no-std environments but usable anywhere.
//!
//! This crate is primarily targeted at drawing GIF frames to [`embedded_graphics`] [`DrawTarget`]s,
//!
//!
//! The minimum supported Rust version for tinygif is nightly `1.67` or greater. Ensure you have the correct
//! version of Rust installed, preferably through <https://rustup.rs>. This dependency on nightly
//! rust isn't of vital importance; if you'd like to use this library with stable rust, please file
//! an issue
//!
//! <!-- README-LINKS
//! [`embedded_graphics`]: https://docs.rs/embedded_graphics
//! [`DrawTarget`]: https://docs.rs/embedded-graphics/latest/embedded_graphics/draw_target/trait.DrawTarget.html
//! [`Image`]: https://docs.rs/embedded-graphics/latest/embedded_graphics/image/struct.Image.html
//! README-LINKS -->
//!
//! [`DrawTarget`]: embedded_graphics::draw_target::DrawTarget
//! [`Image`]: embedded_graphics::image::Image
//! [color types]: embedded_graphics::pixelcolor#structs
//! [header information]: RawBmp::header
//! [color table]: RawBmp::color_table
//! [
//!

//#![deny(missing_docs)]
#![feature(iter_next_chunk)]
#![feature(iter_advance_by)]
#![cfg_attr(not(test), no_std)]
use embedded_graphics::{pixelcolor::Rgb565, prelude::*, primitives::Rectangle};
pub use iterators::SeekableIter;

mod common;
mod frame;
mod header;
mod iterators;
mod parser;
#[cfg(test)]
mod test_utils;

pub use frame::{GifFrame, GifFrameStreamer};

impl<S> ImageDrawable for GifFrameStreamer<S>
where
    S: SeekableIter,
{
    type Color = Rgb565;
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let frame = self.current_frame().unwrap();
        let area = frame.frame_area();
        target.fill_contiguous(&area, frame)
    }

    fn draw_sub_image<D>(&self, _target: &mut D, _area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        todo!()
    }
}

impl<S> OriginDimensions for GifFrameStreamer<S>
where
    S: SeekableIter,
{
    fn size(&self) -> Size {
        self.base_size()
    }
}
