# `waitx`

Provides an extremely low-latency, pure signaling primitive.

## Overview
`waitx` implements an extremely fast, minimal notification mechanism for SPSC/MPSC use cases where only a “wake up” or “event occurred” signal is needed.

## Usage

```rust
fn main() {
    // create the channel
    let (tx, rx) = waitx::unit_channel();

    // Receiver
    let handle = std::thread::spawn(move || {
        // needs to be called because receiving channel
        // should now reference this newly spawned thread.
        rx.update_thread();

        loop {
            //
            // do some work
            //
            rx.recv(); // wait for the sender
        }
    });

    // Sender
    loop {
        //
        // do some work
        //
        tx.send(); // notify the receiver
    }
}
```

## Remarks
- The `waitx::UnitReceiver` parks the thread which it resides in. If moved to another thread after instantiation, its shared handle needs to be updated using its `update_thread` method.
- Parking is unfacilitated. Unparking the receiver's residing thread at any time **will** wake the receiver.

## Benchmarks
Very basic benchmarking was performed locally on a 12th Gen Intel(R) Core(TM) i7-12700K.


![Violin Plot](docs/criterion/ping_pong/report/violin.svg)

View the full benchmark report comparing other popular crates [here](https://ejsch03.github.io/waitx/criterion/report/index.html).