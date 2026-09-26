# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/sagi21805/groupoid/releases/tag/groupoid_macros-v0.1.0) - 2026-09-26

### Other

- group_impl takes state = and rejects associated type bounds on the state
- group_impl binds the state's associated type to its group's
- renamed macros and trait
- clearer error messages
- refactored, and made all transformations to go through restate with clearer error messages
- Changed line and comment length
- refactor
- more refactoring
- kept refactoring
- reorganized, and pushed generic syn extensions to another file
- started refactoring claude code
- commmited unverified tests by claude.
- commited still unverified claude changes.
- refactored
- add extend dependency
- Started refactoring, mainly adding extension methods to syn types
- Handled falliability of the new method on TypeStateArg
- used claude to generate some tests
- add unfinished changes to typestate
- modified for crates io
- Add readme
- improved and restricted typestate structs to implement the sized traits of groupoid and include only one state field
- add the option for #[size(N)] attribute on the group type, which is also captured in a trait.
- restricted blueprint to have only one type definition
- extended test
- used naming module
- forced group methods to take self as an argument, and put the helper trait inside a mod item
- cowrote with claude a working group implementation
- add naming conventions
- updated tests
- new state macro that implements the state trait
- renamed to typestate
- group now implement the group trait, and the generic implementation for gourp traits now require the WithState trait
- add groupoid to dev dependencies so tests would work
- Add tests module
- renamed to groupoid_macros
- moved tests
- reorganized

### Removed

- removed unused mod definitions
- removed prototypes
