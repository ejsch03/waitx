use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::{Duration, Instant};
use waitx::*;

#[allow(unused)]
struct Payload {
    a: i32,
    b: f64,
    c: String,
}

impl Default for Payload {
    fn default() -> Self {
        Self {
            a: 0xBEEF,
            b: f64::from_bits(0x4009_21FB_5444_2D18),
            c: "bowser".to_string(),
        }
    }
}

fn bench_oneshot_ping_pong(c: &mut Criterion) {
    let mut group = c.benchmark_group("oneshot_ping_pong");

    group
        .sample_size(150)
        .measurement_time(Duration::from_secs(45))
        .warm_up_time(Duration::from_secs(5))
        .noise_threshold(0.005)
        .significance_level(0.01)
        .nresamples(200_000);

    group.bench_function("01_waitx", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx1, rx1) = channel::<Payload>();
            let (tx2, rx2) = channel::<Payload>();

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = thread::spawn(move || {
                rx1.update_thread();
                ready_tx.send(Payload::default()).unwrap();

                for _ in 0..iters {
                    rx1.recv();
                    tx2.send(Payload::default());
                }
            });

            ready_rx.recv().unwrap();
            let start = Instant::now();

            for _ in 0..iters {
                tx1.send(Payload::default());
                rx2.recv();
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("02_crossbeam", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx1, rx1) = crossbeam_channel::bounded::<Payload>(0);
            let (tx2, rx2) = crossbeam_channel::bounded::<Payload>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = thread::spawn(move || {
                ready_tx.send(Payload::default()).unwrap();

                for _ in 0..iters {
                    rx1.recv().unwrap();
                    tx2.send(Payload::default()).unwrap();
                }
            });

            ready_rx.recv().unwrap();
            let start = Instant::now();

            for _ in 0..iters {
                tx1.send(Payload::default()).unwrap();
                rx2.recv().unwrap();
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("03_flume", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx1, rx1) = flume::bounded::<Payload>(0);
            let (tx2, rx2) = flume::bounded::<Payload>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = thread::spawn(move || {
                ready_tx.send(Payload::default()).unwrap();

                for _ in 0..iters {
                    rx1.recv().unwrap();
                    tx2.send(Payload::default()).unwrap();
                }
            });

            ready_rx.recv().unwrap();
            let start = Instant::now();

            for _ in 0..iters {
                tx1.send(Payload::default()).unwrap();
                rx2.recv().unwrap();
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("04_std_mpsc", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx1, rx1) = std_mpsc::sync_channel::<Payload>(0);
            let (tx2, rx2) = std_mpsc::sync_channel::<Payload>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = thread::spawn(move || {
                ready_tx.send(Payload::default()).unwrap();

                for _ in 0..iters {
                    rx1.recv().unwrap();
                    tx2.send(Payload::default()).unwrap();
                }
            });

            ready_rx.recv().unwrap();
            let start = Instant::now();

            for _ in 0..iters {
                tx1.send(Payload::default()).unwrap();
                rx2.recv().unwrap();
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.finish();
}

criterion_group!(benches, bench_oneshot_ping_pong);
criterion_main!(benches);
