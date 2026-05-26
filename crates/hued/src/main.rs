use hue_mediaremote::{MediaEvent, MediaRemote};

fn main() {
    let mr = MediaRemote;
    mr.register_events(on_event)
        .expect("failed to register event handler");

    mr.run_loop();
}

fn on_event(event: MediaEvent) {
    match event {
        MediaEvent::Playback { playing } => {
            println!("playback changed: {playing:?}")
        }
        MediaEvent::TrackChanged(t) => {
            println!("track changed:\n{t:#?}")
        }
        MediaEvent::Metadata(m) => {
            println!("metadata refresh:\n{m:#?}")
        }
    }
}
