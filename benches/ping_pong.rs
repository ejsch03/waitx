use atomic_wait::{wait, wake_one};
use criterion::{Criterion, criterion_group, criterion_main};
use event_listener::{Event, Listener};
use spin::Mutex;
use std::sync::{Arc, mpsc as std_mpsc};
use std::time::{Duration, Instant};

fn bench_ping_pong(c: &mut Criterion) {
    let mut group = c.benchmark_group("ping_pong");

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

            let (tx_1, rx_1) = waitx::unit_channel();
            let (tx_2, rx_2) = waitx::unit_channel();

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                // required for waitx
                rx_1.update_thread();

                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    rx_1.recv();
                    tx_2.send();
                }
            });

            // required for waitx
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

    group.bench_function("02_spin", |b| {
        use std::sync::Arc;

        b.iter_custom(|iters| {
            let iters = iters as usize;

            let flag_a_to_b = Arc::new(Mutex::new(false));
            let flag_b_to_a = Arc::new(Mutex::new(false));

            let f1 = flag_a_to_b.clone();
            let f2 = flag_b_to_a.clone();

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    // Wait for A → B
                    loop {
                        if *f1.lock() {
                            break;
                        }
                        std::hint::spin_loop();
                    }
                    *f1.lock() = false;

                    // Send B → A
                    *f2.lock() = true;
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();

            for _ in 0..iters {
                // Send A → B
                *flag_a_to_b.lock() = true;

                // Wait for B → A
                loop {
                    if *flag_b_to_a.lock() {
                        break;
                    }
                    std::hint::spin_loop();
                }
                *flag_b_to_a.lock() = false;
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("03_atomic-wait", |b| {
        use std::sync::atomic::{AtomicU32, Ordering};
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let a_to_b = Arc::new(AtomicU32::new(0));
            let b_to_a = Arc::new(AtomicU32::new(0));

            let (a_to_b_clone, b_to_a_clone) = (a_to_b.clone(), b_to_a.clone());
            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    // Wait for A → B
                    while a_to_b_clone.load(Ordering::Acquire) == 0 {
                        wait(&a_to_b_clone, 0);
                    }
                    a_to_b_clone.store(0, Ordering::Release);

                    // Send B → A
                    b_to_a_clone.store(1, Ordering::Release);
                    wake_one(&*b_to_a_clone);
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();

            for _ in 0..iters {
                // Send A → B
                a_to_b.store(1, Ordering::Release);
                wake_one(&*a_to_b);

                // Wait for B → A
                while b_to_a.load(Ordering::Acquire) == 0 {
                    wait(&b_to_a, 0);
                }
                b_to_a.store(0, Ordering::Release);
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("04_event-listener", |b| {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        b.iter_custom(|iters| {
            let iters = iters as usize;

            let flag_a_to_b = Arc::new(AtomicBool::new(false));
            let flag_b_to_a = Arc::new(AtomicBool::new(false));

            let event_a_to_b = Arc::new(Event::new());
            let event_b_to_a = Arc::new(Event::new());

            let fa = flag_a_to_b.clone();
            let fb = flag_b_to_a.clone();
            let ea = event_a_to_b.clone();
            let eb = event_b_to_a.clone();

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    // Wait for A → B
                    while !fa.load(Ordering::Acquire) {
                        let listener = ea.listen();
                        if !fa.load(Ordering::Acquire) {
                            listener.wait();
                        }
                    }
                    fa.store(false, Ordering::Release);

                    // Send B → A
                    fb.store(true, Ordering::Release);
                    eb.notify(1);
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();

            for _ in 0..iters {
                // Send A → B
                flag_a_to_b.store(true, Ordering::Release);
                event_a_to_b.notify(1);

                // Wait for B → A
                while !flag_b_to_a.load(Ordering::Acquire) {
                    let listener = event_b_to_a.listen();
                    if !flag_b_to_a.load(Ordering::Acquire) {
                        listener.wait();
                    }
                }
                flag_b_to_a.store(false, Ordering::Release);
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("05_crossbeam", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = crossbeam_channel::bounded::<()>(0);
            let (tx_2, rx_2) = crossbeam_channel::bounded::<()>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    // Wait for A → B
                    rx_1.recv().unwrap();
                    // Send B → A
                    tx_2.send(()).unwrap();
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();

            for _ in 0..iters {
                // Send A → B
                tx_1.send(()).unwrap();
                // Wait for B → A
                rx_2.recv().unwrap();
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("06_flume", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = flume::bounded::<()>(0);
            let (tx_2, rx_2) = flume::bounded::<()>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    // Wait for A → B
                    rx_1.recv().unwrap();
                    // Send B → A
                    tx_2.send(()).unwrap();
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();

            for _ in 0..iters {
                // Send A → B
                tx_1.send(()).unwrap();
                // Wait for B → A
                rx_2.recv().unwrap();
            }

            let elapsed = start.elapsed();
            worker.join().unwrap();
            elapsed
        })
    });

    group.bench_function("07_mpsc", |b| {
        b.iter_custom(|iters| {
            let iters = iters as usize;

            let (tx_1, rx_1) = std_mpsc::sync_channel::<()>(0);
            let (tx_2, rx_2) = std_mpsc::sync_channel::<()>(0);

            let (ready_tx, ready_rx) = std_mpsc::channel();

            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();

                for _ in 0..iters {
                    // Wait for A → B
                    rx_1.recv().unwrap();
                    // Send B → A
                    tx_2.send(()).unwrap();
                }
            });

            ready_rx.recv().unwrap();

            let start = Instant::now();

            for _ in 0..iters {
                // Send A → B
                tx_1.send(()).unwrap();
                // Wait for B → A
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
