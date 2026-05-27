#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    WouldBlock,
    Disconnected,
    BufferTooSmall,
    WriteFailed,
    ReadFailed,
}
