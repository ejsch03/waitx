# `waitx`

A lightweight & low-latency, pure signaling primitive.

## Overview
`waitx` implements a minimal but fast notification mechanism for SPSC/MPSC use cases.

## Benchmarks
Performed locally on a 12th Gen Intel(R) Core(TM) i7-12700K.

![Violin Plot](docs/criterion/ping_pong/report/violin.svg)

Full benchmark report [here](https://ejsch03.github.io/waitx/criterion/report/index.html).