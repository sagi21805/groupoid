//! The pipeline with one `#[typestate]` struct and `typestate-groups`.

use core::net::Ipv4Addr;

use typestate_groups::{
    Isomorphic, MorphFrom, Morphic, group, group_impl, group_trait, state,
    state_types, typestate,
};

#[state_types]
pub trait Wire {
    type Addr;
    type Port;
}

#[state]
pub struct Received;
#[state]
pub struct Routed;
#[state]
pub struct Logged;

#[group(Bytes)]
impl Wire for (Received,) {
    #[size(4)]
    type Addr = [u8; 4];
    #[size(2)]
    type Port = [u8; 2];
}

#[group(Native)]
impl Wire for (Routed,) {
    #[size(4)]
    type Addr = u32;
    #[size(2)]
    type Port = u16;
}

#[group(Typed)]
impl Wire for (Logged,) {
    type Addr = Ipv4Addr;
    type Port = u16;
}

#[typestate(unsafe_transmute = true, align = 4)]
pub struct Header<S: Wire> {
    pub dst: S::Addr,
    pub port: S::Port,
    pub ttl: u8,
}

/// Sum of the destination and port, per group.
#[group_trait(by = Wire)]
pub trait Checksum {
    fn checksum(&self) -> u32;
}

#[group_impl(Bytes)]
impl<S: Wire> Checksum for Header<S> {
    fn checksum(&self) -> u32 {
        let [a, b, c, d] = self.dst.map(u32::from);
        let [hi, lo] = self.port.map(u32::from);
        a + b + c + d + (hi << 8 | lo)
    }
}

#[group_impl(Native)]
impl<S: Wire> Checksum for Header<S> {
    fn checksum(&self) -> u32 {
        let [a, b, c, d] = self.dst.to_ne_bytes().map(u32::from);
        a + b + c + d + u32::from(u16::from_be(self.port))
    }
}

impl MorphFrom<Header<Routed>> for Header<Logged> {
    fn morph_from(header: Header<Routed>) -> Self {
        Header {
            dst: Ipv4Addr::from(header.dst.to_ne_bytes()),
            port: u16::from_be(header.port),
            ttl: header.ttl,
        }
    }
}

#[unsafe(no_mangle)]
pub fn groups_transmute(header: Header<Received>) -> Header<Routed> {
    // SAFETY: every bit pattern of `[u8; 4]` and `[u8; 2]` is a valid
    // `u32` and `u16`.
    unsafe { header.transmute_state::<Routed>() }
}

#[unsafe(no_mangle)]
pub fn groups_transmute_ref(header: &Header<Received>) -> &Header<Routed> {
    // SAFETY: as in `groups_transmute`.
    unsafe { header.transmute_state_ref::<Routed>() }
}

#[unsafe(no_mangle)]
pub fn groups_transmute_mut(
    header: &mut Header<Received>,
) -> &mut Header<Routed> {
    // SAFETY: as in `groups_transmute`.
    unsafe { header.transmute_state_mut::<Routed>() }
}

#[unsafe(no_mangle)]
pub fn groups_morph(header: Header<Routed>) -> Header<Logged> {
    header.morph::<Logged>()
}

#[unsafe(no_mangle)]
pub fn groups_checksum_received(header: &Header<Received>) -> u32 {
    header.checksum()
}

#[unsafe(no_mangle)]
pub fn groups_checksum_routed(header: &Header<Routed>) -> u32 {
    header.checksum()
}

#[unsafe(no_mangle)]
pub fn groups_pipeline(
    dst: [u8; 4],
    port: [u8; 2],
    ttl: u8,
) -> Header<Logged> {
    let header = Header::<Received> { dst, port, ttl };
    // SAFETY: as in `groups_transmute`.
    let header = unsafe { header.transmute_state::<Routed>() };
    header.morph::<Logged>()
}
