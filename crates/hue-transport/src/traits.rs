pub trait HueTx {
    type Error;

    fn send(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

pub trait HueRx {
    type Error;

    fn recv(&mut self, out: &mut [u8]) -> Result<usize, Self::Error>;
}

pub trait HueTransport: HueRx + HueTx {}

impl<T> HueTransport for T where T: HueTx + HueRx {}
