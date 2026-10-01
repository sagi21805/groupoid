//! The pipeline with one struct per state and hand-written transitions.

use core::net::Ipv4Addr;

/// A header as it came off the wire.
#[repr(C, align(4))]
pub struct Received {
    pub dst: [u8; 4],
    pub port: [u8; 2],
    pub ttl: u8,
}

/// A header with native integers of the same sizes as [`Received`].
#[repr(C, align(4))]
pub struct Routed {
    pub dst: u32,
    pub port: u16,
    pub ttl: u8,
}

/// A header with typed fields.
#[repr(C, align(4))]
pub struct Logged {
    pub dst: Ipv4Addr,
    pub port: u16,
    pub ttl: u8,
}

const _: () = {
    assert!(size_of::<Received>() == size_of::<Routed>());
    assert!(align_of::<Received>() == align_of::<Routed>());
};

/// Sum of the destination and port, per state.
pub trait Checksum {
    fn checksum(&self) -> u32;
}

impl Checksum for Received {
    fn checksum(&self) -> u32 {
        let [a, b, c, d] = self.dst.map(u32::from);
        let [hi, lo] = self.port.map(u32::from);
        a + b + c + d + (hi << 8 | lo)
    }
}

impl Checksum for Routed {
    fn checksum(&self) -> u32 {
        let [a, b, c, d] = self.dst.to_ne_bytes().map(u32::from);
        a + b + c + d + u32::from(u16::from_be(self.port))
    }
}

impl Received {
    /// # Safety
    ///
    /// Every bit pattern of the fields is a valid [`Routed`] field.
    pub unsafe fn into_routed(self) -> Routed {
        // SAFETY: both structs are `repr(C)` with the same size and
        // alignment, asserted above.
        unsafe { core::mem::transmute::<Received, Routed>(self) }
    }

    /// # Safety
    ///
    /// Every bit pattern of the fields is a valid [`Routed`] field.
    pub unsafe fn as_routed(&self) -> &Routed {
        // SAFETY: as in `into_routed`, and the borrow keeps `self`'s
        // lifetime.
        unsafe { &*(self as *const Received as *const Routed) }
    }

    /// # Safety
    ///
    /// Every bit pattern of the fields is a valid [`Routed`] field.
    pub unsafe fn as_routed_mut(&mut self) -> &mut Routed {
        // SAFETY: as in `into_routed`, and the borrow keeps `self`'s
        // lifetime and uniqueness.
        unsafe { &mut *(self as *mut Received as *mut Routed) }
    }
}

impl From<Routed> for Logged {
    fn from(header: Routed) -> Self {
        Logged {
            dst: Ipv4Addr::from(header.dst.to_ne_bytes()),
            port: u16::from_be(header.port),
            ttl: header.ttl,
        }
    }
}

#[unsafe(no_mangle)]
pub fn hand_transmute(header: Received) -> Routed {
    // SAFETY: every bit pattern of `[u8; 4]` and `[u8; 2]` is a valid
    // `u32` and `u16`.
    unsafe { header.into_routed() }
}

#[unsafe(no_mangle)]
pub fn hand_transmute_ref(header: &Received) -> &Routed {
    // SAFETY: as in `hand_transmute`.
    unsafe { header.as_routed() }
}

#[unsafe(no_mangle)]
pub fn hand_transmute_mut(header: &mut Received) -> &mut Routed {
    // SAFETY: as in `hand_transmute`.
    unsafe { header.as_routed_mut() }
}

#[unsafe(no_mangle)]
pub fn hand_morph(header: Routed) -> Logged {
    header.into()
}

#[unsafe(no_mangle)]
pub fn hand_checksum_received(header: &Received) -> u32 {
    header.checksum()
}

#[unsafe(no_mangle)]
pub fn hand_checksum_routed(header: &Routed) -> u32 {
    header.checksum()
}

#[unsafe(no_mangle)]
pub fn hand_pipeline(dst: [u8; 4], port: [u8; 2], ttl: u8) -> Logged {
    let header = Received { dst, port, ttl };
    // SAFETY: as in `hand_transmute`.
    let header = unsafe { header.into_routed() };
    header.into()
}
