use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use std::{fs::File, os::raw::c_int, path::Path};

use criterion::profiler::Profiler;
use pprof::ProfilerGuard;
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

pub struct FlamegraphProfiler<'a> {
    frequency: c_int,
    active_profiler: Option<ProfilerGuard<'a>>,
}

impl<'a> FlamegraphProfiler<'a> {
    #[allow(dead_code)]
    pub fn new(frequency: c_int) -> Self {
        FlamegraphProfiler {
            frequency,
            active_profiler: None,
        }
    }
}

impl<'a> Profiler for FlamegraphProfiler<'a> {
    fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
        self.active_profiler = Some(ProfilerGuard::new(self.frequency).unwrap());
    }

    fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
        std::fs::create_dir_all(benchmark_dir).unwrap();
        let flamegraph_path = benchmark_dir.join("flamegraph.svg");
        let flamegraph_file = File::create(&flamegraph_path)
            .expect("File system error while creating flamegraph.svg");
        if let Some(profiler) = self.active_profiler.take() {
            profiler
                .report()
                .build()
                .unwrap()
                .flamegraph(flamegraph_file)
                .expect("Error writing flamegraph");
        }
    }
}
