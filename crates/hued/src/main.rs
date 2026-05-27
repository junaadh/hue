use hue_core::state::UiState;
use hue_protocol::{
    builder::FrameBuilder,
    parser::{FeedResult, FrameParser},
    wire::MessageType,
};
use hue_transport::traits::{HueRx, HueTx};
use hued::transport::CdcTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cdc = CdcTransport::open_first()?;

    let mut old = UiState::new();

    let mut new = UiState::new();

    new.track_id = 1;

    new.title.set(b"Nights")?;
    new.artist.set(b"Frank Ocean")?;
    new.album.set(b"Blonde")?;
    new.app_name.set(b"Music")?;

    new.position_ms = 42_000;
    new.duration_ms = 300_000;
    new.playing = true;

    let mut frame = [0u8; 512];

    let len = FrameBuilder::state_delta(&mut frame, 1, &old, &new)?;

    println!("sending state delta: {} bytes", len);
    cdc.send(&frame[..len])?;
    old = new;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        // let n = cdc.recv(&mut rx)?;

        // for &byte in &rx[..n] {
        //     match parser.feed_byte(byte) {
        //         FeedResult::NeedMore => {}

        //         FeedResult::Error(err) => println!("parser error: {err:?}"),

        //         FeedResult::FrameReady => {
        //             let frame =
        //                 parser.take_frame().expect("no parsed frame found");

        //             println!(
        //                 "rx frame: type={:?}, seq={}. payload_len={}",
        //                 frame.header.msg_type,
        //                 frame.header.seq,
        //                 frame.payload.len()
        //             );

        //             if frame.header.msg_type == MessageType::Ping {
        //                 println!("echoed ping frame received");
        //                 return Ok(());
        //             }
        //         }
        //     }
        // }
    }
}
