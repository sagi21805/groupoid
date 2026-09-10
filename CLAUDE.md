# Project Overview

This project is meant to create grouping types for Rust. These types while used with the typestate pattern allow to implement a function over multiple states at once, and declaring a different function with an entirly different action based on the state of the type.

For example if we have a trait the defines certain type, we can implement a method for every type that has a state that it type is a usize. Then we can create a different implementation for the same method for a type that has a state that is a String. This allows us to have a single function that can handle multiple states of a type, while still maintaining type safety and clarity in our code.