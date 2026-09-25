# Project Overview

This project is meant to create grouping types for Rust. These types while used with the typestate pattern allow to implement a function over multiple states at once, and declaring a different function with an entirly different action based on the state of the type.

For example if we have a trait the defines certain type, we can implement a method for every type that has a state that it type is a usize. Then we can create a different implementation for the same method for a type that has a state that is a String. This allows us to have a single function that can handle multiple states of a type, while still maintaining type safety and clarity in our code.

## Rust style

Load the `rust-style` skill before writing, editing or reviewing any Rust code in this project. It holds the owner's conventions and the proc-macro guidance this crate follows. 

When I ask you to rewrite certain code in a way that I like more, find a way to generalize it, and then ask for my permission to include it in the `rust-style` skill. This way, we can keep the codebase consistent and maintainable.

## Unslop

When writing any text, use the `unslop` skill to remove any unnecessary words, phrases, or sentences. This will help keep the text clear.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
