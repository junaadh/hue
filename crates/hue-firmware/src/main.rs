#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    gpio::{Level, Output, Speed},
    rcc::{AHBPrescaler, APBPrescaler, mux},
    spi::{Config as SpiConfig, Mode, Phase, Polarity, Spi},
    time::Hertz,
    usb::InterruptHandler,
};
use embassy_usb::{
    Builder as UsbBuilder, Config as UsbConfig,
    class::cdc_acm::{CdcAcmClass, State as CdcState},
};
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, Triangle},
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use hue_core::state::UiState;
use hue_protocol::{
    builder::FrameBuilder,
    parser::{FeedResult, FrameParser},
    payload::{
        ArtworkChunkPayload, FIELD_ARTIST, FIELD_ARTWORK_ID, FIELD_DURATION,
        FIELD_PLAYING, FIELD_POSITION, FIELD_TITLE, decode_state_delta,
    },
    wire::MessageType,
};
use mipidsi::{
    Builder, interface::SpiInterface, models::ILI9341Rgb565,
    options::Orientation,
};
use panic_halt as _;
use static_cell::StaticCell;

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
    let mut cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);

    let usb = builder.build();

    _spawner.spawn(usb_task(usb).unwrap());

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

    const COVER_X: i32 = 40;
    const COVER_Y: i32 = 18;
    const COVER_W: u32 = 160;
    const COVER_H: u32 = 160;

    const TEXT_X: i32 = 20;
    const TEXT_Y: i32 = 186;
    const TEXT_W: u32 = 200;
    const TEXT_H: u32 = 38;

    const TITLE_Y: i32 = 198;
    const ARTIST_Y: i32 = 214;

    const BAR_X: i32 = 20;
    const BAR_Y: i32 = 234;
    const BAR_W: u32 = 200;
    const BAR_H: u32 = 5;

    const TIME_Y: i32 = 253;
    const TIME_AREA_Y: i32 = 243;
    const TIME_AREA_H: u32 = 20;
    const TIME_LEFT_X: i32 = 20;
    const TIME_RIGHT_X: i32 = 192;

    const BUTTONS_Y: i32 = 267;
    const BTN_SIZE: u32 = 35;

    const PREV_X: i32 = 47;
    const PLAY_X: i32 = 103;
    const NEXT_X: i32 = 159;

    const PREV_TEXT_X: i32 = PREV_X + 14;
    const NEXT_TEXT_X: i32 = NEXT_X + 14;
    const BTN_TEXT_Y: i32 = BUTTONS_Y + 23;

    let bg = Rgb565::new(3, 4, 6);
    let card = Rgb565::new(10, 12, 16);
    let muted = Rgb565::new(28, 30, 34);
    let white = Rgb565::WHITE;
    let accent = Rgb565::new(31, 8, 18);

    let title_style = MonoTextStyle::new(&FONT_6X10, white);
    let artist_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(18, 20, 23));

    display.clear(bg).unwrap();

    Rectangle::new(Point::new(COVER_X, COVER_Y), Size::new(COVER_W, COVER_H))
        .into_styled(PrimitiveStyle::with_fill(card))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(TEXT_X, TEXT_Y), Size::new(TEXT_W, TEXT_H))
        .into_styled(PrimitiveStyle::with_fill(bg))
        .draw(&mut display)
        .unwrap();

    Text::new("Song Name", Point::new(87, TITLE_Y), title_style)
        .draw(&mut display)
        .unwrap();

    Text::new("Artist Name", Point::new(82, ARTIST_Y), artist_style)
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(BAR_X, BAR_Y), Size::new(BAR_W, BAR_H))
        .into_styled(PrimitiveStyle::with_fill(muted))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(
        Point::new(TIME_LEFT_X, TIME_AREA_Y),
        Size::new(200, TIME_AREA_H),
    )
    .into_styled(PrimitiveStyle::with_fill(bg))
    .draw(&mut display)
    .unwrap();

    Text::new("0:00", Point::new(TIME_LEFT_X, TIME_Y), artist_style)
        .draw(&mut display)
        .unwrap();

    Text::new("0:00", Point::new(TIME_RIGHT_X, TIME_Y), artist_style)
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(PREV_X, BUTTONS_Y), BTN_SIZE)
        .into_styled(PrimitiveStyle::with_fill(muted))
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(PLAY_X, BUTTONS_Y), BTN_SIZE)
        .into_styled(PrimitiveStyle::with_fill(muted))
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(NEXT_X, BUTTONS_Y), BTN_SIZE)
        .into_styled(PrimitiveStyle::with_fill(muted))
        .draw(&mut display)
        .unwrap();

    Text::new("<", Point::new(PREV_TEXT_X, BTN_TEXT_Y), title_style)
        .draw(&mut display)
        .unwrap();

    Triangle::new(
        Point::new(116, 279),
        Point::new(116, 292),
        Point::new(128, 285),
    )
    .into_styled(PrimitiveStyle::with_fill(white))
    .draw(&mut display)
    .unwrap();

    Text::new(">", Point::new(NEXT_TEXT_X, BTN_TEXT_Y), title_style)
        .draw(&mut display)
        .unwrap();

    let mut ui_state = UiState::default();
    let mut rx_buf = [0u8; 64];
    let mut tx_buf = [0u8; 64];
    let mut parser = FrameParser::new();
    let mut tx_seq: u16 = 1;

    loop {
        cdc.wait_connection().await;

        loop {
            let n = match cdc.read_packet(&mut rx_buf).await {
                Ok(n) => n,
                Err(_) => break,
            };

            for &byte in &rx_buf[..n] {
                match parser.feed_byte(byte) {
                    FeedResult::NeedMore => {}
                    FeedResult::Error(_) => {}
                    FeedResult::FrameReady => {
                        let frame = parser.take_frame().unwrap();

                        match frame.header.msg_type {
                            MessageType::Ping => {
                                if let Ok(len) =
                                    FrameBuilder::pong(&mut tx_buf, tx_seq)
                                {
                                    tx_seq = tx_seq.wrapping_add(1);
                                    let _ =
                                        cdc.write_packet(&tx_buf[..len]).await;
                                }
                            }

                            MessageType::StateDelta => {
                                let Ok(mask) = decode_state_delta(
                                    frame.payload,
                                    &mut ui_state,
                                ) else {
                                    continue;
                                };
                                let state = ui_state;

                                if mask & FIELD_ARTWORK_ID != 0 {
                                    Rectangle::new(
                                        Point::new(COVER_X, COVER_Y),
                                        Size::new(COVER_W, COVER_H),
                                    )
                                    .into_styled(PrimitiveStyle::with_fill(
                                        Rgb565::new(31, 0, 0),
                                    ))
                                    .draw(&mut display)
                                    .unwrap();
                                }

                                if mask & (FIELD_TITLE | FIELD_ARTIST) != 0 {
                                    Rectangle::new(
                                        Point::new(TEXT_X, TEXT_Y),
                                        Size::new(TEXT_W, TEXT_H),
                                    )
                                    .into_styled(PrimitiveStyle::with_fill(bg))
                                    .draw(&mut display)
                                    .unwrap();

                                    Text::new(
                                        state.title.as_str(),
                                        Point::new(TEXT_X, TITLE_Y),
                                        title_style,
                                    )
                                    .draw(&mut display)
                                    .unwrap();

                                    Text::new(
                                        state.artist.as_str(),
                                        Point::new(TEXT_X, ARTIST_Y),
                                        artist_style,
                                    )
                                    .draw(&mut display)
                                    .unwrap();
                                }

                                if mask & (FIELD_POSITION | FIELD_DURATION) != 0
                                {
                                    Rectangle::new(
                                        Point::new(BAR_X, BAR_Y),
                                        Size::new(BAR_W, BAR_H),
                                    )
                                    .into_styled(PrimitiveStyle::with_fill(
                                        muted,
                                    ))
                                    .draw(&mut display)
                                    .unwrap();

                                    let progress = if state.duration_ms == 0 {
                                        0
                                    } else {
                                        ((state.position_ms * BAR_W as u64)
                                            / state.duration_ms)
                                            as u32
                                    };

                                    Rectangle::new(
                                        Point::new(BAR_X, BAR_Y),
                                        Size::new(progress.min(BAR_W), BAR_H),
                                    )
                                    .into_styled(PrimitiveStyle::with_fill(
                                        accent,
                                    ))
                                    .draw(&mut display)
                                    .unwrap();

                                    Rectangle::new(
                                        Point::new(TIME_LEFT_X, TIME_AREA_Y),
                                        Size::new(200, TIME_AREA_H),
                                    )
                                    .into_styled(PrimitiveStyle::with_fill(bg))
                                    .draw(&mut display)
                                    .unwrap();

                                    let mut left = [0u8; 6];
                                    let mut right = [0u8; 6];

                                    Text::new(
                                        seconds_text(
                                            state.position_ms,
                                            &mut left,
                                        ),
                                        Point::new(TIME_LEFT_X, TIME_Y),
                                        artist_style,
                                    )
                                    .draw(&mut display)
                                    .unwrap();

                                    Text::new(
                                        seconds_text(
                                            state.duration_ms,
                                            &mut right,
                                        ),
                                        Point::new(TIME_RIGHT_X, TIME_Y),
                                        artist_style,
                                    )
                                    .draw(&mut display)
                                    .unwrap();
                                }

                                if mask & FIELD_PLAYING != 0 {
                                    Circle::new(
                                        Point::new(PLAY_X, BUTTONS_Y),
                                        BTN_SIZE,
                                    )
                                    .into_styled(PrimitiveStyle::with_fill(
                                        muted,
                                    ))
                                    .draw(&mut display)
                                    .unwrap();

                                    if state.playing {
                                        Rectangle::new(
                                            Point::new(115, 278),
                                            Size::new(4, 14),
                                        )
                                        .into_styled(PrimitiveStyle::with_fill(
                                            white,
                                        ))
                                        .draw(&mut display)
                                        .unwrap();

                                        Rectangle::new(
                                            Point::new(123, 278),
                                            Size::new(4, 14),
                                        )
                                        .into_styled(PrimitiveStyle::with_fill(
                                            white,
                                        ))
                                        .draw(&mut display)
                                        .unwrap();
                                    } else {
                                        Triangle::new(
                                            Point::new(116, 279),
                                            Point::new(116, 292),
                                            Point::new(128, 285),
                                        )
                                        .into_styled(PrimitiveStyle::with_fill(
                                            white,
                                        ))
                                        .draw(&mut display)
                                        .unwrap();
                                    }
                                }
                            }

                            MessageType::ArtworkChunk => {
                                if let Ok(chunk) =
                                    ArtworkChunkPayload::decode(frame.payload)
                                {
                                    if chunk.data.len() == 320 {
                                        let raw = ImageRawLE::<Rgb565>::new(
                                            chunk.data, 160,
                                        );
                                        Image::new(
                                            &raw,
                                            Point::new(
                                                COVER_X,
                                                COVER_Y
                                                    + chunk.chunk_index as i32,
                                            ),
                                        )
                                        .draw(&mut display)
                                        .unwrap();
                                    }
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

fn seconds_text(ms: u64, out: &mut [u8; 6]) -> &str {
    let total = ms / 1000;
    let min = total / 60;
    let sec = total % 60;

    out[0] = b'0' + ((min / 10) % 10) as u8;
    out[1] = b'0' + (min % 10) as u8;
    out[2] = b':';
    out[3] = b'0' + (sec / 10) as u8;
    out[4] = b'0' + (sec % 10) as u8;
    out[5] = 0;

    unsafe { core::str::from_utf8_unchecked(&out[..5]) }
}
