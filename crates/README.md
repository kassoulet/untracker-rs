# Untracker Crates

This directory contains the forked and improved Rust bindings for `libopenmpt`.

## `openmpt-sys`

Low-level FFI bindings for `libopenmpt`.
- Automatically links against system `libopenmpt` using `pkg-config`.
- Includes support for the `libopenmpt_ext.h` extension API.

## `openmpt`

High-level, safe Rust bindings for `libopenmpt`.
- Fixed memory safety issues in module disposal.
- Added `ext` module for interacting with the `libopenmpt` extension API.
- Modernized to Rust 2021 edition.
