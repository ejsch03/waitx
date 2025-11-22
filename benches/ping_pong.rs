use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};

fn bench_ping_pong(c: &mut Criterion) {
    let mut group = c.benchmark_group("ping_pong");

    group
        .sample_size(100)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5))
        .noise_threshold(0.02)
        .significance_level(0.01)
        .nresamples(80_000);

    // --------------------------------------------------------
    // waitx::unit_channel()
    // --------------------------------------------------------
    group.bench_function("waitx", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = waitx::unit_channel();
            let (tx_2, rx_2) = waitx::unit_channel();

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                rx_1.update_thread(); // this is required

                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    rx_1.recv();
                    tx_2.send();
                }
            });
            rx_2.update_thread();

            ready_rx.recv().unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                tx_1.send();
                rx_2.recv();
            }
            let elapsed = start.elapsed();

            worker.join().unwrap();
            elapsed
        })
    });

    // --------------------------------------------------------
    // crossbeam_channel::bounded(0)
    // --------------------------------------------------------
    group.bench_function("crossbeam_bounded_0", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = crossbeam_channel::bounded::<()>(0);
            let (tx_2, rx_2) = crossbeam_channel::bounded::<()>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    rx_1.recv().unwrap();
                    tx_2.send(()).unwrap();
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                tx_1.send(()).unwrap();
                rx_2.recv().unwrap();
            }
            let elapsed = start.elapsed();

            worker.join().unwrap();
            elapsed
        })
    });

    // --------------------------------------------------------
    // flume::bounded(0)
    // --------------------------------------------------------
    group.bench_function("flume_bounded_0", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = flume::bounded::<()>(0);
            let (tx_2, rx_2) = flume::bounded::<()>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    rx_1.recv().unwrap();
                    tx_2.send(()).unwrap();
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                tx_1.send(()).unwrap();
                rx_2.recv().unwrap();
            }
            let elapsed = start.elapsed();

            worker.join().unwrap();
            elapsed
        })
    });

    // --------------------------------------------------------
    // std::sync::mpsc::sync_channel(0)
    // --------------------------------------------------------
    group.bench_function("std_mpsc_sync_0", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = std_mpsc::sync_channel::<()>(0);
            let (tx_2, rx_2) = std_mpsc::sync_channel::<()>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    rx_1.recv().unwrap();
                    tx_2.send(()).unwrap();
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();
            for _ in 0..iters {
                tx_1.send(()).unwrap();
                rx_2.recv().unwrap();
            }
            let elapsed = start.elapsed();

            worker.join().unwrap();
            elapsed
        })
    });

    group.finish();
}

criterion_group!(benches, bench_ping_pong);
criterion_main!(benches);
