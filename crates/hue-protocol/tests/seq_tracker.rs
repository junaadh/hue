use hue_protocol::{SeqStatus, SeqTracker, seq_distance};

#[test]
fn seq_tracker_accepts_expected_sequence_and_advances() {
    let mut tracker = SeqTracker::new(10);

    assert_eq!(tracker.expected(), 10);
    assert_eq!(tracker.observe(10), SeqStatus::Expected);
    assert_eq!(tracker.expected(), 11);
    assert_eq!(tracker.observe(11), SeqStatus::Expected);
    assert_eq!(tracker.expected(), 12);
}

#[test]
fn seq_tracker_reports_gap_without_advancing() {
    let mut tracker = SeqTracker::new(10);

    assert_eq!(
        tracker.observe(12),
        SeqStatus::Gap {
            expected: 10,
            received: 12,
        }
    );
    assert_eq!(tracker.expected(), 10);
}

#[test]
fn seq_tracker_reports_duplicate_without_advancing() {
    let mut tracker = SeqTracker::new(10);

    assert_eq!(tracker.observe(10), SeqStatus::Expected);
    assert_eq!(
        tracker.observe(10),
        SeqStatus::Duplicate {
            expected: 11,
            received: 10,
        }
    );
    assert_eq!(tracker.expected(), 11);
}

#[test]
fn seq_tracker_handles_wraparound() {
    let mut tracker = SeqTracker::new(u16::MAX);

    assert_eq!(tracker.observe(u16::MAX), SeqStatus::Expected);
    assert_eq!(tracker.expected(), 0);
    assert_eq!(tracker.observe(0), SeqStatus::Expected);
    assert_eq!(tracker.expected(), 1);
}

#[test]
fn seq_tracker_classifies_old_wrapped_sequence_as_duplicate() {
    let mut tracker = SeqTracker::new(0);

    assert_eq!(
        tracker.observe(u16::MAX),
        SeqStatus::Duplicate {
            expected: 0,
            received: u16::MAX,
        }
    );
    assert_eq!(tracker.expected(), 0);
}

#[test]
fn seq_distance_root_api_remains_available() {
    assert_eq!(seq_distance(0, u16::MAX), 1);
    assert_eq!(seq_distance(41, 42), -1);
}
