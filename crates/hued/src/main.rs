use hue_protocol::{
    builder::FrameBuilder,
    parser::{FeedResult, FrameParser},
    wire::MessageType,
};
use hue_transport::traits::{HueRx, HueTx};
use hued::transport::CdcTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cdc = CdcTransport::open_first()?;

    let mut tx = [0u8; 64];
    let len = FrameBuilder::ping(&mut tx, 1)?;

    cdc.send(&tx[..len])?;
    println!("sent ping frame: {} bytes", len);

    let mut parser = FrameParser::new();
    let mut rx = [0u8; 64];

    loop {
        let n = cdc.recv(&mut rx)?;

        for &byte in &rx[..n] {
            match parser.feed_byte(byte) {
                FeedResult::NeedMore => {}

                FeedResult::Error(err) => println!("parser error: {err:?}"),

                FeedResult::FrameReady => {
                    let frame =
                        parser.take_frame().expect("no parsed frame found");

                    println!(
                        "rx frame: type={:?}, seq={}. payload_len={}",
                        frame.header.msg_type,
                        frame.header.seq,
                        frame.payload.len()
                    );

                    if frame.header.msg_type == MessageType::Ping {
                        println!("echoed ping frame received");
                        return Ok(());
                    }
                }
            }
        }
    }
}
