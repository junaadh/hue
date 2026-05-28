use hue_core::state::UiState;
use hue_mediaremote::{MediaEvent, MediaRemote};
use hue_protocol::{
    builder::FrameBuilder,
    payload::{
        ARTWORK_FORMAT_RGB565, ArtworkBeginPayload, ArtworkChunkPayload,
        ArtworkEndPayload,
    },
};
use hue_transport::traits::HueTx;
use hued::{
    artwork::{ARTWORK_SIZE, process_artwork},
    transport::CdcTransport,
};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

fn snapshot_to_state(snapshot: &hue_mediaremote::PlaybackSnapshot) -> UiState {
    let mut state = UiState::new();

    state.title.truncate(snapshot.song.title.as_bytes());
    state.artist.truncate(snapshot.song.artist.as_bytes());
    state.album.truncate(snapshot.song.album.as_bytes());
    state.app_name.truncate(snapshot.song.app_name.as_bytes());

    state.position_ms = (snapshot.progress.position * 1000.0) as u64;
    state.duration_ms = (snapshot.progress.duration * 1000.0) as u64;
    state.playing = snapshot.playing;

    state.recompute_track_id();
    state
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cdc = CdcTransport::open_first()?;

    let shared = Arc::new(Mutex::new(BridgeState {
        cdc,
        old: UiState::new(),
        seq: 1,
    }));

    let remote = MediaRemote;

    {
        let shared = shared.clone();

        remote.register_events(move |event| {
            let snapshot = match event {
                MediaEvent::TrackChanged(snapshot) => snapshot,
                MediaEvent::Metadata(snapshot) => snapshot,

                MediaEvent::Playback { playing } => {
                    let mut guard = shared.lock().unwrap();

                    let mut new = guard.old;
                    new.playing = playing;

                    let _ = send_state(&mut guard, &new);
                    return;
                }
            };

            let mut new = snapshot_to_state(&snapshot);

            let artwork = snapshot
                .artwork_path
                .as_deref()
                .and_then(|path| process_artwork(path).ok());

            if let Some(artwork) = &artwork {
                println!(
                    "artwork: {}x{} bytes={}",
                    artwork.width,
                    artwork.height,
                    artwork.rgb565.len()
                );

                new.artwork_id = artwork.artwork_id;
            }

            let mut guard = shared.lock().unwrap();

            let _ = send_state(&mut guard, &new);

            if let Some(artwork) = &artwork {
                let _ = send_artwork(&mut guard, artwork);
            }
        })?;
    }

    remote.run_loop();
}

struct BridgeState {
    cdc: CdcTransport,
    old: UiState,
    seq: u16,
}

fn send_state(
    state: &mut BridgeState,
    new: &UiState,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frame = [0u8; 512];

    let len =
        FrameBuilder::state_delta(&mut frame, state.seq, &state.old, new)?;

    if len > 0 {
        state.cdc.send(&frame[..len])?;
        state.seq = state.seq.wrapping_add(1);
        state.old = *new;
    }

    Ok(())
}

use hue_core::hash::Fnv32;

const ARTWORK_CHUNK_DATA: usize = ARTWORK_SIZE as usize * 2; // one RGB565 row

fn send_artwork(
    state: &mut BridgeState,
    artwork: &hued::artwork::ProcessedArtwork,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frame = [0u8; 512];

    let mut hash = Fnv32::new();
    hash.feed_bytes(&artwork.rgb565);

    let checksum = hash.finish();

    let begin = ArtworkBeginPayload {
        artwork_id: artwork.artwork_id,
        width: artwork.width,
        height: artwork.height,
        total_bytes: artwork.rgb565.len() as u32,
        format: ARTWORK_FORMAT_RGB565,
    };

    let len = FrameBuilder::artwork_begin(&mut frame, state.seq, begin)?;

    state.seq = state.seq.wrapping_add(1);

    state.cdc.send(&frame[..len])?;

    for (i, data) in artwork.rgb565.chunks(ARTWORK_CHUNK_DATA).enumerate() {
        let chunk = ArtworkChunkPayload {
            artwork_id: artwork.artwork_id,
            chunk_index: i as u16,
            data,
        };

        let len = FrameBuilder::artwork_chunk(&mut frame, state.seq, chunk)?;

        state.seq = state.seq.wrapping_add(1);

        state.cdc.send(&frame[..len])?;
        thread::sleep(Duration::from_millis(10));
    }

    let end = ArtworkEndPayload {
        artwork_id: artwork.artwork_id,
        checksum,
    };

    let len = FrameBuilder::artwork_end(&mut frame, state.seq, end)?;

    state.seq = state.seq.wrapping_add(1);

    state.cdc.send(&frame[..len])?;

    Ok(())
}
