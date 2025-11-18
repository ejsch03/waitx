use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::thread::spawn;
use std::time::Instant;

const ITERS: u64 = 100_000;

// -------------------- waitx::unit_channel --------------------

fn ping_pong_waitx() {
    use waitx::unit_channel;

    let (tx_ab, rx_ab) = unit_channel();
    let (tx_ba, rx_ba) = unit_channel();

    let pong = spawn(move || {
        rx_ab.update_thread();
        for _ in 0..ITERS {
            rx_ab.recv();
            tx_ba.send();
        }
    });

    let ping = spawn(move || {
        rx_ba.update_thread();
        let start = Instant::now();
        for _ in 0..ITERS {
            tx_ab.send();
            rx_ba.recv();
        }
        start.elapsed()
    });

    let total = ping.join().unwrap();
    pong.join().unwrap();

    std::hint::black_box(total);
}

// -------------------- std::sync::mpsc::sync_channel(0) --------------------

fn ping_pong_std_mpsc() {
    use std::sync::mpsc::sync_channel;

    let (tx_ab, rx_ab) = sync_channel::<()>(0);
    let (tx_ba, rx_ba) = sync_channel::<()>(0);

    let pong = spawn(move || {
        for _ in 0..ITERS {
            rx_ab.recv().unwrap();
            tx_ba.send(()).unwrap();
        }
    });

    let ping = spawn(move || {
        let start = Instant::now();
        for _ in 0..ITERS {
            tx_ab.send(()).unwrap();
            rx_ba.recv().unwrap();
        }
        start.elapsed()
    });

    let total = ping.join().unwrap();
    pong.join().unwrap();

    std::hint::black_box(total);
}

// -------------------- crossbeam_channel::bounded(0) --------------------

fn ping_pong_crossbeam() {
    use crossbeam_channel::bounded;

    let (tx_ab, rx_ab) = bounded::<()>(0);
    let (tx_ba, rx_ba) = bounded::<()>(0);

    let pong = spawn(move || {
        for _ in 0..ITERS {
            rx_ab.recv().unwrap();
            tx_ba.send(()).unwrap();
        }
    });

    let ping = spawn(move || {
        let start = Instant::now();
        for _ in 0..ITERS {
            tx_ab.send(()).unwrap();
            rx_ba.recv().unwrap();
        }
        start.elapsed()
    });

    let total = ping.join().unwrap();
    pong.join().unwrap();

    std::hint::black_box(total);
}

// -------------------- flume::bounded(0) --------------------

fn ping_pong_flume() {
    use flume::bounded;

    let (tx_ab, rx_ab) = bounded::<()>(0);
    let (tx_ba, rx_ba) = bounded::<()>(0);

    let pong = spawn(move || {
        for _ in 0..ITERS {
            rx_ab.recv().unwrap();
            tx_ba.send(()).unwrap();
        }
    });

    let ping = spawn(move || {
        let start = Instant::now();
        for _ in 0..ITERS {
            tx_ab.send(()).unwrap();
            rx_ba.recv().unwrap();
        }
        start.elapsed()
    });

    let total = ping.join().unwrap();
    pong.join().unwrap();

    std::hint::black_box(total);
}

// -------------------- Criterion harness --------------------

fn bench_channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_ping_pong");
    group.sample_size(20); // tweak if needed

    group.bench_function("waitx_unit_channel", |b| {
        b.iter_batched(|| (), |_| ping_pong_waitx(), BatchSize::SmallInput)
    });

    group.bench_function("std_mpsc_sync_channel_0", |b| {
        b.iter_batched(|| (), |_| ping_pong_std_mpsc(), BatchSize::SmallInput)
    });

    group.bench_function("crossbeam_bounded_0", |b| {
        b.iter_batched(|| (), |_| ping_pong_crossbeam(), BatchSize::SmallInput)
    });

    group.bench_function("flume_bounded_0", |b| {
        b.iter_batched(|| (), |_| ping_pong_flume(), BatchSize::SmallInput)
    });

    group.finish();
}

criterion_group!(benches, bench_channels);
criterion_main!(benches);
