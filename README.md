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

An order moves through four states. While it's open, it holds item
prices. Once it's paid, it holds a receipt. You want one `total()` that
works in every state.

In plain Rust you'd reach for two impls:

```rust,ignore
impl<S: Stage<Contents = Vec<u32>>> Order<S> { fn total(&self) -> u32 { .. } }
impl<S: Stage<Contents = Receipt>> Order<S> { fn total(&self) -> u32 { .. } }
```

rustc rejects this with `E0592: duplicate definitions`, because it can't
tell that no state satisfies both bounds. So you end up with one impl per
state, or an enum and a runtime `match`. With `groupoid`, you name the
groups and write one impl for each:

```rust
use groupoid::{group, group_impl, group_trait, state, template, typestate};

struct Receipt {
    charged: u32,
}

#[template]
trait Stage {
    type Contents;
}

#[state]
struct Cart;
#[state]
struct Checkout;
#[state]
struct Paid;
#[state]
struct Shipped;

#[group(Open)]
impl Stage for (Cart, Checkout) {
    type Contents = Vec<u32>;
}

#[group(Closed)]
impl Stage for (Paid, Shipped) {
    type Contents = Receipt;
}

#[typestate]
struct Order<S: Stage> {
    contents: S::Contents,
}

#[group_trait(by = Stage)]
trait Total {
    fn total(&self) -> u32;
}

#[group_impl(Open)]
impl<S: Stage> Total for Order<S> {
    fn total(&self) -> u32 {
        self.contents.iter().sum()
    }
}

#[group_impl(Closed)]
impl<S: Stage> Total for Order<S> {
    fn total(&self) -> u32 {
        self.contents.charged
    }
}

fn main() {
    let checkout = Order::<Checkout> { contents: vec![1200, 300] };
    assert_eq!(checkout.total(), 1500);

    let paid: Order<Paid> = checkout.morph_with(|prices| Receipt {
        charged: prices.iter().sum(),
    });
    assert_eq!(paid.total(), 1500);
}
```

Each impl sees the concrete field type, so `self.contents.iter()` needs
no cast. A new state joins a group by being added to its tuple, and it
gets `total()` without another line of code.

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
