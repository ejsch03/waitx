#[cfg(test)]
mod loom_tests {
    use crate::channel::channel;
    use crate::pair::pair;
    use crate::util::Tuning;
    use loom::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use loom::thread;

    /// required for loom since it cannot model busy spin loops.
    const PARK_ONLY: Tuning = Tuning::new(0, 0);

    /// signal arrives before wait
    #[test]
    fn pair_signal_before_wait() {
        loom::model(|| {
            let (waker, waiter) = pair();
            waker.signal();
            waiter.wait_with_tuning(PARK_ONLY);
        });
    }

    /// waiter parks first; signal arrives from a concurrent thread and unparks it.
    #[test]
    fn pair_wait_then_signal() {
        loom::model(|| {
            let (waker, waiter) = pair();
            let waker = Arc::new(waker);
            let w = waker.clone();

            let t = thread::spawn(move || {
                w.signal();
            });

            waiter.wait_with_tuning(PARK_ONLY);
            t.join().unwrap();
        });
    }

    /// two sequential signals, two sequential waits.
    #[test]
    fn pair_two_signals_two_waits() {
        loom::model(|| {
            let (waker, waiter) = pair();
            let waker = Arc::new(waker);
            let w = waker.clone();

            let t = thread::spawn(move || {
                w.signal();
                w.signal();
            });

            waiter.wait_with_tuning(PARK_ONLY);
            waiter.wait_with_tuning(PARK_ONLY);
            t.join().unwrap();
        });
    }

    /// `try_wait` succeeds immediately when the signal has already been sent.
    #[test]
    fn pair_try_wait_after_signal() {
        loom::model(|| {
            let (waker, waiter) = pair();
            waker.signal();
            assert!(waiter.try_wait());
        });
    }

    /// `try_wait` returns false when no signal has been sent.
    #[test]
    fn pair_try_wait_miss() {
        loom::model(|| {
            let (_waker, waiter) = pair();
            assert!(!waiter.try_wait());
        });
    }

    /// `try_wait` and `signal` are concurrent.
    #[test]
    fn pair_try_wait_races_signal() {
        loom::model(|| {
            let (waker, waiter) = pair();
            let waker = Arc::new(waker);
            let w = waker.clone();

            let t = thread::spawn(move || {
                w.signal();
            });

            if !waiter.try_wait() {
                // signal arrived after our check; fall back to blocking wait.
                waiter.wait_with_tuning(PARK_ONLY);
            }

            t.join().unwrap();
        });
    }

    /// exposes the TOCTOU liveness bug in `wake()`.
    #[test]
    fn pair_wake_liveness_race() {
        loom::model(|| {
            let (waker, waiter) = pair();
            let waker = Arc::new(waker);
            let w = waker.clone();

            let t = thread::spawn(move || {
                w.wake();
            });

            waiter.wait_with_tuning(PARK_ONLY);
            t.join().unwrap();
        });
    }

    /// sequential send then recv on the same thread.
    #[test]
    fn channel_sequential_send_recv() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();
            tx.send(42);
            assert_eq!(rx.recv(), 42);
        });
    }

    /// receiver parks waiting for the slot to fill.
    #[test]
    fn channel_recv_parks_then_send() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();

            let t = thread::spawn(move || rx.recv());

            tx.send(7);
            assert_eq!(t.join().unwrap(), 7);
        });
    }

    /// sender parks waiting for the slot to empty; receiver consumes concurrently.
    #[test]
    fn channel_send_parks_waiting_for_drain() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();

            let t = thread::spawn(move || {
                tx.send(1);
                tx.send(2); // must park until receiver drains slot
            });

            assert_eq!(rx.recv(), 1);
            assert_eq!(rx.recv(), 2);
            t.join().unwrap();
        });
    }

    /// two round trips
    #[test]
    fn channel_ping_pong_two_rounds() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();

            let t = thread::spawn(move || {
                tx.send(1);
                tx.send(2);
            });

            assert_eq!(rx.recv(), 1);
            assert_eq!(rx.recv(), 2);
            t.join().unwrap();
        });
    }

    /// `try_send` must return `Err` if the slot is already full.
    #[test]
    fn channel_try_send_full_slot() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();
            tx.send(1); // fill the slot
            assert!(tx.try_send(2).is_err());
            assert_eq!(rx.recv(), 1);
        });
    }

    /// `try_recv` must return `None` when the slot is empty.
    #[test]
    fn channel_try_recv_empty_slot() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();
            assert!(rx.try_recv().is_none());
            tx.send(1);
            assert_eq!(rx.try_recv(), Some(1));
        });
    }

    /// concurrent `try_send` and `try_recv` with both sides on separate threads.
    #[test]
    fn channel_try_send_try_recv_concurrent() {
        loom::model(|| {
            let (tx, rx) = channel::<u8>();
            let sent = Arc::new(loom::sync::atomic::AtomicBool::new(false));
            let sent2 = sent.clone();

            let t = thread::spawn(move || {
                if tx.try_send(42).is_ok() {
                    sent2.store(true, Ordering::Release);
                }
            });

            let received = rx.try_recv();
            t.join().unwrap();

            // if the sender succeeded, the receiver must have gotten the value
            if sent.load(Ordering::Acquire) {
                match received {
                    Some(v) => assert_eq!(v, 42),
                    None => assert_eq!(rx.recv(), 42),
                }
            }
        });
    }

    /// a value sent but never received must be dropped when the channel is dropped.
    #[test]
    fn channel_drop_unreceived_value() {
        loom::model(|| {
            let drops = Arc::new(AtomicUsize::new(0));

            struct Probe(Arc<AtomicUsize>);
            impl Drop for Probe {
                fn drop(&mut self) {
                    self.0.fetch_add(1, Ordering::Relaxed);
                }
            }

            let (tx, rx) = channel::<Probe>();

            let d = drops.clone();
            let t = thread::spawn(move || {
                tx.send(Probe(d));
            });

            t.join().unwrap();
            drop(rx); // drop without calling recv

            assert_eq!(drops.load(Ordering::SeqCst), 1);
        });
    }

    /// a received value must be dropped exactly once.
    #[test]
    fn channel_drop_received_value_exactly_once() {
        loom::model(|| {
            let drops = Arc::new(AtomicUsize::new(0));

            struct Probe(Arc<AtomicUsize>);
            impl Drop for Probe {
                fn drop(&mut self) {
                    self.0.fetch_add(1, Ordering::Relaxed);
                }
            }

            let (tx, rx) = channel::<Probe>();
            tx.send(Probe(drops.clone()));
            drop(rx.recv());

            assert_eq!(drops.load(Ordering::SeqCst), 1);
        });
    }

    /// validates that the `Slot`'s `mark_full` Release and `recv`'s `is_full`.
    #[test]
    fn channel_slot_write_visible_after_recv() {
        loom::model(|| {
            let (tx, rx) = channel::<[u8; 4]>();

            let t = thread::spawn(move || {
                tx.send([1, 2, 3, 4]);
            });

            assert_eq!(rx.recv(), [1, 2, 3, 4]);
            t.join().unwrap();
        });
    }
}
