pub mod error;
#[cfg(target_os = "macos")]
pub mod mac_cdc;

pub use error::*;
#[cfg(target_os = "macos")]
pub use mac_cdc::*;

pub type Result<T> = core::result::Result<T, error::CdcError>;
