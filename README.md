# typestate-groups

[![crates.io](https://img.shields.io/crates/v/typestate-groups.svg)](https://crates.io/crates/typestate-groups)
[![docs.rs](https://docs.rs/typestate-groups/badge.svg)](https://docs.rs/typestate-groups)
[![CI](https://github.com/sagi21805/typestate-groups/actions/workflows/ci.yml/badge.svg)](https://github.com/sagi21805/typestate-groups/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Typestate-based grouping types for Rust.

Group the states of a typestate by the types they carry, then implement
a trait once per group. Every state in the group gets that impl, and the
impl sees the group's concrete types.

## Grouping states

A sensor frame holds raw ADC counts (`u16`) in two states and volts
(`f32`) in a third. `Report` gets one impl per group:

```rust
use typestate_groups::{group, group_impl, group_trait, state, state_types, typestate};

#[state_types]
trait Stage {
    type Sample;
}

#[state]
struct Sampled;
#[state]
struct Filtered;
#[state]
struct Calibrated;

#[group(Counts)]
impl Stage for (Sampled, Filtered) {
    type Sample = u16;
}

#[group(Volts)]
impl Stage for (Calibrated,) {
    type Sample = f32;
}

#[typestate]
struct Frame<S: Stage> {
    sensor: u32,
    range: [S::Sample; 2],
}

#[group_trait(by = Stage)]
trait Report {
    fn report(&self) -> String;
}

#[group_impl(Counts)]
impl<S: Stage> Report for Frame<S> {
    fn report(&self) -> String {
        let [lo, hi] = self.range;
        format!("sensor {}: {lo}..={hi} counts", self.sensor)
    }
}

#[group_impl(Volts)]
impl<S: Stage> Report for Frame<S> {
    fn report(&self) -> String {
        let [lo, hi] = self.range;
        format!("sensor {}: {lo:.2}..={hi:.2} V", self.sensor)
    }
}

fn main() {
    let raw = Frame::<Sampled> {
        sensor: 7,
        range: [0, 4095],
    };
    assert_eq!(raw.report(), "sensor 7: 0..=4095 counts");

    let filtered = Frame::<Filtered> {
        sensor: 7,
        range: [12, 4000],
    };
    assert_eq!(filtered.report(), "sensor 7: 12..=4000 counts");

    let calibrated = Frame::<Calibrated> {
        sensor: 7,
        range: [0.0, 3.3],
    };
    assert_eq!(calibrated.report(), "sensor 7: 0.00..=3.30 V");
}
```

Plain Rust rejects this pair with `E0119: conflicting implementations`,
because coherence ignores associated types:

```rust,ignore
impl<S: Stage<Sample = u16>> Report for Frame<S> { .. }
impl<S: Stage<Sample = f32>> Report for Frame<S> { .. }
```

With `typestate-groups`, `{lo}` is a `u16` in one impl and `{lo:.2}` formats an
`f32` in the other. Add a state to a group's tuple and it has `report()`
with no new code.

## Changing state

A `#[state_types]` trait can declare several associated types. A packet header below
carries an address and a port, and moves through three states:

- `Received` holds the raw bytes.
- `Routed` holds native integers of the same sizes.
- `Logged` holds an `Ipv4Addr` and a host-order port.

`Received` to `Routed` reinterprets the bits of both fields in place with
`cast_state`.
`Routed` to `Logged` builds new values, so it goes through `MorphFrom`.

```rust
use core::net::Ipv4Addr;

use typestate_groups::{
    Isomorphic, MorphFrom, Morphic, group, state, state_types, typestate,
};

#[state_types]
trait Wire {
    type Addr;
    type Port;
}

#[state]
struct Received;
#[state]
struct Routed;
#[state]
struct Logged;

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
struct Header<S: Wire> {
    dst: S::Addr,
    port: S::Port,
    ttl: u8,
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

fn main() {
    let header = Header::<Received> {
        dst: [10, 0, 0, 2],
        port: [0x1f, 0x90],
        ttl: 64,
    };

    let header = header.cast_state::<Routed>();
    assert_eq!(header.dst, u32::from_ne_bytes([10, 0, 0, 2]));

    let header = header.morph::<Logged>();
    assert_eq!(header.dst, Ipv4Addr::new(10, 0, 0, 2));
    assert_eq!((header.port, header.ttl), (8080, 64));
}
```

`#[size(N)]` pins a type's size, and `unsafe_transmute = true` generates
`cast_state` and `transmute_state` for every pair of states whose fields
line up. Both checks run at compile time. Change `Routed`'s address to `u64` and the
build fails with:

```text
error[E0080]: evaluation panicked: change `#[size(4)]` on `Addr` to the size of `u64`
  --> src/main.rs:28:1
   |
28 | #[group(Native)]
   | ^^^^^^^^^^^^^^^^ evaluation of `_` failed here
```

Transmuting straight to `Logged` fails too, with `add #[size(N)] to the
associated type set to Ipv4Addr in #[group(Typed)] to transmute it`.
Drop `align = 4` and the error asks for it back, since `[u8; 4]` and
`u32` disagree on alignment.

`cast_state` also checks that every field which changes type holds bits
valid in the new state, using [`zerocopy`](https://docs.rs/zerocopy): the
old type must be `IntoBytes` (no padding) and the new one `FromBytes`
(every bit pattern valid). Casting a `u8` into a `bool` fails with
``convert with `morph`: `u8` may hold bits that are not a valid `bool` ``.
Derive those traits on your own field types to cast them.

| Method | Each changed field also needs |
|---|---|
| `cast_state` | nothing more |
| `cast_state_mut` | the same check from the new type back to the old |
| `cast_state_ref` | `zerocopy::Immutable` on both types, so no `Cell` |

When a field type is valid only for some values, such as `u32` into
`char`, use `unsafe { transmute_state() }` and run `cargo miri test` on
the code that calls it.

`morph::<Target>()` calls your `MorphFrom` impl. One impl can be generic
over both states, and a container can morph its fields with their own
impls. For a conversion that can fail, implement `TryMorphFrom` and call
`try_morph::<Target>()`.

## Crates

- [`typestate-groups`](typestate-groups) — the public API.
- [`typestate-groups-macros`](typestate-groups-macros) — procedural macros powering `typestate-groups`.

## Installation

```toml
[dependencies]
typestate-groups = "0.2"
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this crate by you, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
