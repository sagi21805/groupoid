# groupoid

[![crates.io](https://img.shields.io/crates/v/groupoid.svg)](https://crates.io/crates/groupoid)
[![docs.rs](https://docs.rs/groupoid/badge.svg)](https://docs.rs/groupoid)
[![CI](https://github.com/sagi21805/groupoid/actions/workflows/ci.yml/badge.svg)](https://github.com/sagi21805/groupoid/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Typestate-based grouping types for Rust.

`groupoid` lets you implement a method once for every state that shares an underlying
type (for example, every state whose value is a `usize`), and a different implementation
for states that share another type (for example `String`) — while the typestate pattern
keeps each state's transitions type-safe and explicit.

## Example

A sensor frame moves through four states. The first two hold raw 12-bit
ADC counts (`u16`), the last two hold volts (`f32`). The frame keeps its
samples in three shapes: a `Vec`, an `Option` and an array.

```rust
use groupoid::{group, group_impl, group_trait, state, template, typestate};

#[template]
trait Stage {
    type Sample;
}

#[state]
struct Sampled;
#[state]
struct Filtered;
#[state]
struct Calibrated;
#[state]
struct Published;

#[group(Counts)]
impl Stage for (Sampled, Filtered) {
    type Sample = u16;
}

#[group(Volts)]
impl Stage for (Calibrated, Published) {
    type Sample = f32;
}

#[typestate]
struct Frame<S: Stage> {
    sensor: u32,
    history: Vec<S::Sample>,
    latest: Option<S::Sample>,
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

impl Frame<Sampled> {
    fn filter(self) -> Frame<Filtered> {
        self.morph_with(|counts| counts.min(4095))
    }
}

impl Frame<Filtered> {
    fn calibrate(self) -> Frame<Calibrated> {
        self.morph_with(|counts| f32::from(counts) * 3.3 / 4095.0)
    }
}

impl Frame<Calibrated> {
    fn publish(self) -> Frame<Published> {
        self.morph_with(|volts| (volts * 100.0).round() / 100.0)
    }
}

fn main() {
    let frame = Frame::<Sampled> {
        sensor: 7,
        history: vec![0, 2048, 9999],
        latest: Some(9999),
        range: [0, 9999],
    };
    assert_eq!(frame.report(), "sensor 7: 0..=9999 counts");

    let frame = frame.filter();
    assert_eq!(frame.report(), "sensor 7: 0..=4095 counts");

    let frame = frame.calibrate().publish();
    assert_eq!(frame.report(), "sensor 7: 0.00..=3.30 V");
    assert_eq!(frame.latest, Some(3.3));
}
```

Three things happen here that plain Rust won't give you.

**Two impls of one trait, split by an associated type.** Plain Rust
rejects this pair with `E0119: conflicting implementations`, because
coherence doesn't look at associated types:

```rust,ignore
impl<S: Stage<Sample = u16>> Report for Frame<S> { .. }
impl<S: Stage<Sample = f32>> Report for Frame<S> { .. }
```

The usual workarounds are one impl per state or an enum with a runtime
`match`. With `groupoid` each group gets one impl, and that impl sees the
concrete type: `{lo}` is a `u16` in one and `{lo:.2}` formats an `f32` in
the other. Add a state to a group's tuple and it has `report()` with no
new code.

**One closure converts every sample.** `calibrate` never names a field.
`morph_with` walks the `Vec`, the `Option` and the array, calls the
closure on each of the six samples, and moves `sensor` across untouched.
Add a `BTreeMap<u8, S::Sample>` field and the three transitions still
compile unchanged. The return type picks the target state, so a closure
that returns the wrong sample type is a type error.

**Transitions check their order.** `filter` exists only on
`Frame<Sampled>` and `calibrate` only on `Frame<Filtered>`, so
`frame.calibrate()` on a raw frame doesn't compile.

### Several associated types

A template can declare more than one associated type. `#[template]` then
also generates `UnitsMorph<Src, Dst>`, with one method per associated
type. Implement it once, on any type, and `morph` applies it to every
struct that uses the template:

```rust
use groupoid::{group, state, template, typestate};

#[template]
trait Units {
    type Temp;
    type Time;
}

#[state]
struct Raw;
#[state]
struct Si;

#[group(Adc)]
impl Units for (Raw,) {
    type Temp = u16;
    type Time = u32;
}

#[group(Metric)]
impl Units for (Si,) {
    type Temp = f32;
    type Time = f64;
}

#[typestate]
struct Sample<S: Units> {
    temp: S::Temp,
    at: S::Time,
}

#[typestate]
struct Log<S: Units> {
    readings: Vec<(S::Time, S::Temp)>,
}

struct Calibration {
    tick_hz: f64,
}

impl UnitsMorph<Raw, Si> for Calibration {
    fn temp(&mut self, counts: u16) -> f32 {
        f32::from(counts) / 10.0 - 40.0
    }

    fn time(&mut self, ticks: u32) -> f64 {
        f64::from(ticks) / self.tick_hz
    }
}

fn main() {
    let mut calibration = Calibration { tick_hz: 1000.0 };

    let sample = Sample::<Raw> { temp: 650, at: 1500 };
    let sample: Sample<Si> = sample.morph(&mut calibration);
    assert_eq!((sample.temp, sample.at), (25.0, 1.5));

    let log = Log::<Raw> {
        readings: vec![(0, 400), (2000, 650)],
    };
    let log: Log<Si> = log.morph(&mut calibration);
    assert_eq!(log.readings, vec![(0.0, 0.0), (2.0, 25.0)]);

    let sample = Sample::<Raw> { temp: 650, at: 1500 };
    let sample: Sample<Si> = sample.morph_with(SampleMorph {
        temp: &mut |counts| f32::from(counts),
        time: &mut |ticks| f64::from(ticks),
    });
    assert_eq!((sample.temp, sample.at), (650.0, 1500.0));
}
```

`&mut calibration` lends the morpher, so one `Calibration` converts both
structs. In `Log`, each tuple in the `Vec` sends its time and its
temperature to the matching method. For a one-off conversion,
`morph_with` takes a `SampleMorph` with one closure per associated type
that `Sample` uses. Each associated type can carry its own `#[size(N)]`,
and `unsafe_transmute = true` needs it only on the types the struct uses.

### Zero-copy transitions

When every state-dependent field is a bare `S::Addr`, a transition can
reuse the bits instead of rebuilding the value. `#[size(N)]` pins each
group's size, and `unsafe_transmute = true` generates the transmute:

```rust
use groupoid::{Isomorphic, group, state, template, typestate};

#[template]
trait Wire {
    type Addr;
}

#[state]
struct Received;
#[state]
struct Routed;

#[group(Octets)]
impl Wire for (Received,) {
    #[size(4)]
    type Addr = [u8; 4];
}

#[group(Packed)]
impl Wire for (Routed,) {
    #[size(4)]
    type Addr = u32;
}

#[typestate(unsafe_transmute = true, align = 4)]
struct Header<S: Wire> {
    src: S::Addr,
    dst: S::Addr,
    ttl: u8,
}

fn main() {
    let header = Header::<Received> {
        src: [10, 0, 0, 1],
        dst: [10, 0, 0, 2],
        ttl: 64,
    };
    // SAFETY: every bit pattern of `[u8; 4]` is a valid `u32`.
    let header: Header<Routed> = unsafe { header.transmute_state() };
    assert_eq!(header.dst, u32::from_ne_bytes([10, 0, 0, 2]));
    assert_eq!(header.ttl, 64);
}
```

The layout is checked at compile time. Change `Packed` to `u64` and the
build fails with an error that says what to write:

```text
error[E0080]: evaluation panicked: change `#[size(4)]` on `Addr` to the size of `u64`
  --> src/main.rs:19:1
   |
19 | #[group(Packed)]
   | ^^^^^^^^^^^^^^^^ evaluation of `_` failed here
```

Drop `align = 4` and the error asks you to add it back, since `[u8; 4]`
and `u32` disagree on alignment.

## Crates

- [`groupoid`](groupoid) — the public API.
- [`groupoid_macros`](groupoid_macros) — procedural macros powering `groupoid`.

## Installation

```toml
[dependencies]
groupoid = "0.1"
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this crate by you, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
