# asm-compare

The packet-header pipeline from the main README, written twice:

- [`src/hand.rs`](src/hand.rs) has one struct per state, an `assert!` on
  their layouts, and `transmute` and pointer casts written by hand.
- [`src/groups.rs`](src/groups.rs) has one `#[typestate]` struct and uses
  `transmute_state`, `morph` and `#[group_impl]`.

Each `hand_*` function has a `groups_*` twin with the same body. The
release x86-64 assembly of both is in [`asm/hand.s`](asm/hand.s) and
[`asm/groups.s`](asm/groups.s).

## Results

| Function           | What it does                                  | hand | groups | Same IR |
|--------------------|-----------------------------------------------|-----:|-------:|:-------:|
| `transmute`        | `Received` to `Routed` by value               |    2 |      2 | yes     |
| `transmute_ref`    | `&Received` to `&Routed`                      |    2 |      2 | yes     |
| `transmute_mut`    | `&mut Received` to `&mut Routed`              |    2 |      2 | yes     |
| `morph`            | `Routed` to `Logged`, swaps the port's bytes  |    8 |      8 | yes     |
| `checksum_received`| group method on the `Bytes` group             |   15 |     15 | no      |
| `checksum_routed`  | group method on the `Native` group            |   15 |     15 | no      |
| `pipeline`         | build, transmute, then morph                  |    9 |      9 | no      |

The hand and groups columns count instructions, `ret` included.

"Same IR" means LLVM found the two functions identical and emitted
`hand_x` as an alias of `groups_x`. The rest use the same instructions in
a different order. The transmutes compile to `mov rax, rdi; ret`: the
layout checks run at compile time and leave nothing behind.

## Regenerating

```sh
asm-compare/dump-asm.sh
```

The script builds the crate with `--release --emit asm` and one codegen
unit, then writes each module's functions with assembler directives
removed.
