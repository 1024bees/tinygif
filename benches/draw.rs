use criterion::{criterion_group, criterion_main, Criterion};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
mod perf;

use tinygif::GifFrameStreamer;

// TODO: use e-g framebuffer when it's added
struct Framebuffer<const WIDTH: usize, const HEIGHT: usize> {
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

fn parser_benchmarks(c: &mut Criterion) {
    c.bench_function("drawing_crab_vec", |b| {
        let crab = include_bytes!("test/crab.gif");

        let mut gif = GifFrameStreamer::from_slice(crab).unwrap();

        b.iter(|| {
            for _ in 0..gif.num_images() {
                let frame = gif.next_frame().unwrap();
                let _: Vec<Rgb565> = frame.into_iter().collect();
            }
            gif.reset()
        })
    });

    c.bench_function("drawing_bee_vec", |b| {
        let crab = include_bytes!("test/bee.gif");

        let mut gif = GifFrameStreamer::from_slice(crab).unwrap();

        b.iter(|| {
            for _ in 0..gif.num_images() {
                let frame = gif.next_frame().unwrap();
                let _: Vec<Rgb565> = frame.into_iter().collect();
            }
            gif.reset()
        })
    });

    c.bench_function("drawing_bee_buffer", |b| {
        let crab = include_bytes!("test/bee.gif");

        let mut gif = GifFrameStreamer::from_slice(crab).unwrap();
        let mut fb = Framebuffer::<240, 240>::new();

        b.iter(|| {
            for _ in 0..gif.num_images() {
                gif.seek_to_next_frame().unwrap();
                Image::new(&gif, Point::zero()).draw(&mut fb).unwrap();
            }
            gif.reset()
        })
    });
}

criterion_group!(name = benches; config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(10000)); targets = parser_benchmarks);
criterion_main!(benches);
