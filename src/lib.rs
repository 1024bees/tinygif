//! A small gif parser primarily for embedded, no-std environments but usable anywhere.
//!
//! This crate is primarily targeted at drawing BMP images to [`embedded_graphics`] [`DrawTarget`]s,

#![deny(missing_docs)]
#![feature(iter_next_chunk)]
#![feature(iter_advance_by)]

use core::marker::PhantomData;

use frame::Frame;

use embedded_graphics::{
    pixelcolor::{
        raw::{RawU1, RawU16, RawU24, RawU32, RawU4, RawU8},
        Rgb555, Rgb565, Rgb888,
    },
    prelude::*,
    primitives::Rectangle,
};

mod common;
mod frame;
mod header;
mod parser;

///Gif public API
pub struct Gif<'a, C> {
    raw_gif: RawGif<'a>,
    color_type: PhantomData<C>,
}

/// Some gif frame idk
pub struct GifFrame<'a, C> {
    frame: Frame<'a>,
    color_type: PhantomData<C>,
}

impl<C> ImageDrawable for GifFrame<'_, C>
where
    C: PixelColor + From<Rgb555> + From<Rgb565> + From<Rgb888>,
{
    type Color = C;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = C>,
    {
        let area = self.bounding_box();

        todo!();
        let pixels = [];
        target.fill_contiguous(&area, pixels);
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        todo!()
    }
}

impl<C> OriginDimensions for GifFrame<'_, C>
where
    C: PixelColor,
{
    fn size(&self) -> Size {
        todo!();
    }
}

impl<'a, C> Gif<'a, C>
where
    C: PixelColor + From<Rgb555> + From<Rgb565> + From<Rgb888>,
{
    /// Creates a gif object from a byte slice.
    ///
    /// The created object keeps a shared reference to the input and does not dynamically allocate
    /// memory.
    pub fn from_slice(bytes: &'a [u8]) -> Result<Self, ()> {
        todo!()
    }
}

/// Gif bytes sins etc
pub struct RawGif<'a> {
    paylaod: &'a [u8],
}
