//! This example displays BMP images using the embedded-graphics simulator.
//!
//! Basic usage: `cargo run --example display -- BMP_FILE`
//!
//! More usage and arguments can be listed by running `cargo run --example display -- --help`

use clap::{ArgEnum, Parser};
use embedded_graphics::{
    image::Image,
    pixelcolor::{BinaryColor, Gray8, Rgb555, Rgb565, Rgb888},
    prelude::*,
};
use embedded_graphics_simulator::{
    OutputSettings, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use std::{fs, num::NonZeroU32, path::PathBuf};
use tinygif::GifFrameStreamer;

#[derive(Parser)]
struct Args {
    /// Pixel scale
    #[clap(long, default_value = "1")]
    scale: NonZeroU32,

    /// BMP file
    bmp_file: PathBuf,
}

fn display_gif(data: &[u8], settings: &OutputSettings) {
    let mut gif = GifFrameStreamer::from_slice(&data).unwrap();

    let mut display = SimulatorDisplay::<Rgb565>::new(gif.size());

    let mut window = Window::new("GIF viewer", &settings);
    loop {
        gif.seek_to_next_frame().unwrap();
        Image::new(&gif, Point::zero())
            .draw(&mut display.color_converted())
            .unwrap();
        window.update(&display);
        if window.events().any(|e| e == SimulatorEvent::Quit) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(gif.delay_ms() as u64));
    }
}

fn main() {
    let args = Args::parse();

    let settings = OutputSettingsBuilder::new()
        .scale(args.scale.into())
        .build();

    let data = fs::read(&args.bmp_file).unwrap();

    display_gif(&data, &settings);
}
