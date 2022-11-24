use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

// TODO: use e-g framebuffer when it's added
pub(crate) struct Framebuffer<const WIDTH: usize, const HEIGHT: usize> {
    pixels: [[Rgb565; WIDTH]; HEIGHT],
}

impl<const WIDTH: usize, const HEIGHT: usize> Framebuffer<WIDTH, HEIGHT> {
    pub fn new() -> Self {
        let color = Rgb565::BLACK;

        Self {
            pixels: [[color; WIDTH]; HEIGHT],
        }
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> DrawTarget for Framebuffer<WIDTH, HEIGHT> {
    type Error = std::convert::Infallible;
    type Color = Rgb565;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Rgb565>>,
    {
        for Pixel(p, c) in pixels {
            self.pixels[p.y as usize][p.x as usize] = c;
        }

        Ok(())
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> OriginDimensions for Framebuffer<WIDTH, HEIGHT> {
    fn size(&self) -> embedded_graphics::prelude::Size {
        Size::new(240, 320)
    }
}
