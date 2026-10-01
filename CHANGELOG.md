# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/sagi21805/groupoid/compare/groupoid-v0.1.1...groupoid-v0.2.0) - 2026-10-01

### Other

- Changed readme for new morph syntax
- Greatly simplified morphing.
- document several associated types, morph and the struct Morph in the readme and macro docs
- add the template Morph trait and morph to apply a reusable morpher
- check every field offset in LAYOUT_CHECK so align = N can't hide a moved field
- small modifications, mainly to decrease doc size.
- Changed trait and file structure to be much more readable, while preserving clear errors
- replace SizedGroup and SizedWithState with per-type layout markers checked by the target state
- allow several associated types in a template and morph each projection with its own closure
- Delete groupoid_macros/test.rs

### Removed

- removed unused struct in test to satisfy clippy

## [0.1.1](https://github.com/sagi21805/groupoid/compare/groupoid-v0.1.0...groupoid-v0.1.1) - 2026-09-26

### Other

- update readme
