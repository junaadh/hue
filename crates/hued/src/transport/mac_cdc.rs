use std::{
    ffi::{CString, c_char, c_int, c_uint, c_void},
    fs, io,
    os::fd::RawFd,
    path::PathBuf,
};

use hue_transport::traits::{HueRx, HueTx};

use crate::transport::CdcError;

const O_RDWR: c_int = 0x0002;
const O_NOCITY: c_int = 0x0002_0000;
const O_NONBLOCK: c_int = 0x0004;

const TCSANOW: c_int = 0;
const B115200: c_uint = 115200;

#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u64,
    c_oflag: u64,
    c_cflag: u64,
    c_lflag: u64,
    c_cc: [u8; 20],
    c_ispeed: u64,
    c_ospeed: u64,
}

unsafe extern "C" {
    fn open(path: *const c_char, flags: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;

    fn tcgetattr(fd: c_int, termios: *mut Termios) -> c_int;
    fn tcsetattr(fd: c_int, action: c_int, termios: *const Termios) -> c_int;
    fn cfmakeraw(termios: *mut Termios);
    fn cfsetspeed(termios: *mut Termios, speed: c_uint) -> c_int;
}

pub struct CdcTransport {
    fd: RawFd,
}

impl CdcTransport {
    pub fn open(path: &str) -> super::Result<Self> {
        let c_path = CString::new(path).map_err(|_| CdcError::InvalidPath)?;

        let fd =
            unsafe { open(c_path.as_ptr(), O_RDWR | O_NOCITY | O_NONBLOCK) };

        if fd < 0 {
            return Err(CdcError::Io(io::Error::last_os_error()));
        }

        let transport = Self { fd };

        if let Err(err) = transport.configure_raw() {
            unsafe {
                close(fd);
            }
            return Err(err);
        }

        Ok(transport)
    }

    pub fn open_first() -> super::Result<Self> {
        let path = find_first_cdc_device()?;
        let path = path.to_str().ok_or(CdcError::InvalidPath)?;

        Self::open(path)
    }

    fn configure_raw(&self) -> super::Result<()> {
        let mut termios = unsafe { core::mem::zeroed::<Termios>() };

        let rc = unsafe { tcgetattr(self.fd, &mut termios) };
        if rc < 0 {
            return Err(CdcError::Io(io::Error::last_os_error()));
        }

        unsafe {
            cfmakeraw(&mut termios);
        }

        let rc = unsafe { cfsetspeed(&mut termios, B115200) };
        if rc < 0 {
            return Err(CdcError::Io(io::Error::last_os_error()));
        }

        let rc = unsafe { tcsetattr(self.fd, TCSANOW, &termios) };
        if rc < 0 {
            return Err(CdcError::Io(io::Error::last_os_error()));
        }

        Ok(())
    }
}

impl Drop for CdcTransport {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
        }
    }
}

impl HueTx for CdcTransport {
    type Error = CdcError;

    fn send(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut written = 0;

        while written < bytes.len() {
            let n = unsafe {
                write(
                    self.fd,
                    bytes[written..].as_ptr().cast(),
                    bytes.len() - written,
                )
            };

            if n < 0 {
                return Err(CdcError::Io(io::Error::last_os_error()));
            }

            if n == 0 {
                return Err(CdcError::Io(io::ErrorKind::WriteZero.into()));
            }

            written += n as usize;
        }

        Ok(())
    }
}

impl HueRx for CdcTransport {
    type Error = CdcError;

    fn recv(&mut self, out: &mut [u8]) -> Result<usize, Self::Error> {
        let n = unsafe { read(self.fd, out.as_mut_ptr().cast(), out.len()) };

        if n < 0 {
            let err = io::Error::last_os_error();

            if err.kind() == io::ErrorKind::WouldBlock {
                return Ok(0);
            }

            return Err(CdcError::Io(err));
        }

        Ok(n as usize)
    }
}

fn find_first_cdc_device() -> super::Result<PathBuf> {
    for entry in fs::read_dir("/dev")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with("cu.usbmodem") || name.starts_with("tty.usbmodem") {
            return Ok(entry.path());
        }
    }

    Err(CdcError::NoDevice)
}
