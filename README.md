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
