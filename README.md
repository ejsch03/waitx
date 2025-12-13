# `waitx`
Lightweight and low-latency signaling & channel primitives.

# Overview
Minimal, low-latency signaling and single-value channels for Rust. Includes Waker/Waiter and Sender/Receiver, beating other crates in microbenchmarks while staying lightweight and safe.

## Benchmarks
Performed locally on a 12th Gen Intel(R) Core(TM) i7-12700K.

![Violin Plot](docs/criterion/oneshot_ping_pong/report/violin.svg)

![Violin Plot](docs/criterion/unit_ping_pong/report/violin.svg)

Full benchmarking report [here](https://ejsch03.github.io/waitx/criterion/report/index.html).

# Todo
- [ ] Finish documentation.