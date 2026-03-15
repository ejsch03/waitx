# `waitx`

Synchronous signaling and single-slot channel primitives.

## Overview

`waitx` provides two minimal building blocks for low-contention workloads:

- `waitx::pair`  
  A synchronous counted blocking notification primitive (`Waker` / `Waiter`).
- `waitx::channel`  
  A synchronous single-slot channel (`Sender` / `Receiver`).

## Benchmarks

Full Criterion report available [here](https://ejsch03.github.io/waitx/criterion/report/index.html). Benchmarked on a Raspberry Pi 5 (Raspberry Pi OS 64-bit).

![Unit Violin Plot](docs/criterion/unit_ping_pong/report/violin.svg)

![Oneshot Violin Plot](docs/criterion/oneshot_ping_pong/report/violin.svg)

## Testing
```sh
# Standard tests
cargo test

# Loom concurrency tests
cargo test --features loom --lib
```
