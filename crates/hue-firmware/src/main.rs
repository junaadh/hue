#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    spi::{Config as SpiConfig, Mode, Phase, Polarity, Spi},
    time::Hertz,
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{
    Builder, interface::SpiInterface, models::ILI9341Rgb565,
    options::Orientation,
};
use panic_halt as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // SPI1
    let mut spi_cfg = SpiConfig::default();
    spi_cfg.frequency = Hertz(10_000_000);
    spi_cfg.mode = Mode {
        polarity: Polarity::IdleHigh,
        phase: Phase::CaptureOnSecondTransition,
    };

    let spi = Spi::new_blocking_txonly(
        p.SPI1, p.PA5, // SCK
        p.PA7, // MOSI
        spi_cfg,
    );

    let cs = Output::new(p.PB0, Level::High, Speed::VeryHigh);

    let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

    let dc = Output::new(p.PB1, Level::Low, Speed::VeryHigh);

    let rst = Output::new(p.PB2, Level::High, Speed::VeryHigh);

    let mut di_buf = [0u8; 512];

    let mut display = Builder::new(
        ILI9341Rgb565,
        SpiInterface::new(spi_dev, dc, &mut di_buf),
    )
    .reset_pin(rst)
    .color_order(mipidsi::options::ColorOrder::Bgr)
    .orientation(Orientation::new().flip_horizontal())
    .init(&mut embassy_time::Delay)
    .unwrap();

    // const W: i32 = 240;
    // const H: i32 = 320;

    let bg = Rgb565::new(3, 4, 6);
    let card = Rgb565::new(10, 12, 16);
    let muted = Rgb565::new(28, 30, 34);
    let white = Rgb565::WHITE;
    let accent = Rgb565::new(31, 8, 18);

    let title_style = MonoTextStyle::new(&FONT_6X10, white);
    let artist_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(18, 20, 23));

    display.clear(bg).unwrap();

    // Artwork: smaller + higher
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(40, 18), Size::new(160, 160)),
        Size::new(14, 14),
    )
    .into_styled(PrimitiveStyle::with_fill(card))
    .draw(&mut display)
    .unwrap();

    Rectangle::new(Point::new(62, 40), Size::new(116, 116))
        .into_styled(PrimitiveStyle::with_stroke(muted, 2))
        .draw(&mut display)
        .unwrap();

    // Text shifted up
    Text::new("Song Title", Point::new(90, 197), title_style)
        .draw(&mut display)
        .unwrap();

    Text::new("Artist", Point::new(102, 217), artist_style)
        .draw(&mut display)
        .unwrap();

    // Progress shifted up
    Rectangle::new(Point::new(25, 242), Size::new(190, 4))
        .into_styled(PrimitiveStyle::with_fill(muted))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(25, 242), Size::new(72, 4))
        .into_styled(PrimitiveStyle::with_fill(accent))
        .draw(&mut display)
        .unwrap();

    // Time labels
    Text::new("01:24", Point::new(25, 264), artist_style)
        .draw(&mut display)
        .unwrap();

    Text::new("-02:16", Point::new(177, 264), artist_style)
        .draw(&mut display)
        .unwrap();

    // Buttons moved safely up
    Text::new("<", Point::new(58, 298), title_style)
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(110, 278), 28)
        .into_styled(PrimitiveStyle::with_fill(card))
        .draw(&mut display)
        .unwrap();

    Text::new("||", Point::new(118, 298), title_style)
        .draw(&mut display)
        .unwrap();

    Text::new(">", Point::new(181, 298), title_style)
        .draw(&mut display)
        .unwrap();

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
