# `waitx` — Synchronous Ultra-Low-Latency Signaling

`waitx` provides a **high-performance, low-latency, empty-channel signaling primitive**.

## Example

```rust
fn main() {
    let (tx, rx) = waitx::unit_channel();

    // Receiver thread
    let handle = std::thread::spawn(move || {
        rx.update_thread(); // must be called once per receiving thread

        loop {
            rx.recv();
            println!("Received at {:?}", std::time::Instant::now());
        }
    });

    // Sender loop
    loop {
        thread::sleep(std::time::Duration::from_millis(10));
        tx.send();
    }

    let _ = handle.join();
}
```