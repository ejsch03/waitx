# `waitx`
Lightweight signaling & channel primitives for low workloads.

# Overview
- A notification primitive (Waker/Waiter).
- A single-slot channel (Sender/Receiver).

## Benchmarks
Performed locally on a i7-12700K (Windows 11).

![Violin Plot](docs/criterion/oneshot_ping_pong/report/violin.svg)

![Violin Plot](docs/criterion/unit_ping_pong/report/violin.svg)

Full benchmarking report [here](https://ejsch03.github.io/waitx/criterion/report/index.html).

# Todo
- [ ] Finish documentation.
