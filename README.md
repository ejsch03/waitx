# `waitx`
Lightweight signaling & channel primitives for low workloads.

# Overview
- A notification primitive (Waker/Waiter).
- A single-slot channel (Sender/Receiver).

## Benchmarks
Benchmarks were run on a Raspberry Pi 5 (Raspberry Pi OS 64-bit), chosen for
its bare-metal stability and low measurement variance. Full report
[here](https://ejsch03.github.io/waitx/criterion/report/index.html).

![Violin Plot](docs/criterion/oneshot_ping_pong/report/violin.svg)

![Violin Plot](docs/criterion/unit_ping_pong/report/violin.svg)