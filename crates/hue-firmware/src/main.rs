#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    gpio::{Level, Output, Speed},
    peripherals::USB_OTG_FS,
    rcc::{AHBPrescaler, APBPrescaler, mux},
    spi::{Config as SpiConfig, Mode, Phase, Polarity, Spi},
    time::Hertz,
    usb::{Driver, InterruptHandler},
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal,
};
use embassy_time::{Duration, Timer};
use embassy_usb::{
    Builder as UsbBuilder, Config as UsbConfig,
    class::cdc_acm::{CdcAcmClass, State as CdcState},
};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use hue_core::state::UiState;
use hue_protocol::{
    builder::FrameBuilder,
    parser::{FeedResult, FrameParser},
    payload::decode_state_delta,
    wire::MessageType,
};
use mipidsi::{
    Builder, interface::SpiInterface, models::ILI9341Rgb565,
    options::Orientation,
};
use panic_halt as _;
use static_cell::StaticCell;

static UI_SIGNAL: Signal<CriticalSectionRawMutex, UiState> = Signal::new();

#[embassy_executor::task]
async fn usb_task(
    mut usb: embassy_usb::UsbDevice<
        'static,
        embassy_stm32::usb::Driver<
            'static,
            embassy_stm32::peripherals::USB_OTG_FS,
        >,
    >,
) {
    usb.run().await;
}

#[embassy_executor::task]
async fn cdc_task(
    mut class: CdcAcmClass<'static, Driver<'static, USB_OTG_FS>>,
) {
    let mut ui_state = UiState::default();

    let mut rx_buf = [0u8; 64];
    let mut tx_buf = [0u8; 64];
    let mut parser = FrameParser::new();
    let mut tx_seq: u16 = 1;

    loop {
        class.wait_connection().await;

        loop {
            let n = match class.read_packet(&mut rx_buf).await {
                Ok(n) => n,
                Err(_) => break,
            };

            for &byte in &rx_buf[..n] {
                match parser.feed_byte(byte) {
                    FeedResult::NeedMore => {}

                    FeedResult::Error(_) => {
                        // Ignore for now.
                    }

                    FeedResult::FrameReady => {
                        let frame = parser.take_frame().unwrap();

                        match frame.header.msg_type {
                            MessageType::Ping => {
                                if let Ok(len) =
                                    FrameBuilder::pong(&mut tx_buf, tx_seq)
                                {
                                    tx_seq = tx_seq.wrapping_add(1);
                                    let _ = class
                                        .write_packet(&tx_buf[..len])
                                        .await;
                                }
                            }

                            MessageType::StateDelta => {
                                if decode_state_delta(
                                    frame.payload,
                                    &mut ui_state,
                                )
                                .is_ok()
                                {
                                    UI_SIGNAL.signal(ui_state.clone());
                                }
                            }

                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

bind_interrupts!(struct Irqs {
    OTG_FS => InterruptHandler<embassy_stm32::peripherals::USB_OTG_FS>;
});

static EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();

static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static MSOS_DESCRIPTOR: StaticCell<[u8; 128]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

static CDC_STATE: StaticCell<CdcState<'static>> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    use embassy_stm32::rcc::{
        Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource,
        Sysclk,
    };

    let mut config = embassy_stm32::Config::default();

    config.rcc.hse = Some(Hse {
        freq: Hertz(25_000_000),
        mode: HseMode::Oscillator,
    });

    config.rcc.pll_src = PllSource::HSE;

    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV25,  // 25 MHz / 25 = 1 MHz
        mul: PllMul::MUL192,       // 1 MHz * 192 = 192 MHz
        divp: Some(PllPDiv::DIV2), // 192 / 2 = 96 MHz SYSCLK
        divq: Some(PllQDiv::DIV4), // 192 / 4 = 48 MHz USB
        divr: None,
    });

    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;

    let p = embassy_stm32::init(config);

    let usb_driver_cfg = embassy_stm32::usb::Config::default();

    let usb_driver = embassy_stm32::usb::Driver::new_fs(
        p.USB_OTG_FS,
        Irqs,
        p.PA12,
        p.PA11,
        EP_OUT_BUFFER.init([0; 256]),
        usb_driver_cfg,
    );

    let mut usb_config = UsbConfig::new(0x1209, 0x0001);
    usb_config.manufacturer = Some("Hue");
    usb_config.product = Some("Hue Now Playing");
    usb_config.serial_number = Some("0001");
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = 64;

    let mut builder = UsbBuilder::new(
        usb_driver,
        usb_config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        MSOS_DESCRIPTOR.init([0; 128]),
        CONTROL_BUF.init([0; 64]),
    );

    let cdc_state = CDC_STATE.init(CdcState::new());
    let cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);

    let usb = builder.build();

    _spawner.spawn(usb_task(usb).unwrap());
    _spawner.spawn(cdc_task(cdc).unwrap());

    // SPI1
    let mut spi_cfg = SpiConfig::default();
    spi_cfg.frequency = Hertz(40_000_000);
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
        let state = UI_SIGNAL.wait().await;

        Rectangle::new(Point::new(40, 190), Size::new(160, 45))
            .into_styled(PrimitiveStyle::with_fill(bg))
            .draw(&mut display)
            .unwrap();

        Text::new(state.title.as_str(), Point::new(70, 197), title_style)
            .draw(&mut display)
            .unwrap();

        Text::new(state.artist.as_str(), Point::new(70, 217), artist_style)
            .draw(&mut display)
            .unwrap();

        Rectangle::new(Point::new(25, 242), Size::new(190, 4))
            .into_styled(PrimitiveStyle::with_fill(muted))
            .draw(&mut display)
            .unwrap();

        let progress = if state.duration_ms == 0 {
            0
        } else {
            ((state.position_ms * 190) / state.duration_ms) as u32
        };

        Rectangle::new(Point::new(25, 242), Size::new(progress.min(190), 4))
            .into_styled(PrimitiveStyle::with_fill(accent))
            .draw(&mut display)
            .unwrap();
    }
}
