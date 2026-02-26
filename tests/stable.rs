#[cfg(all(test, not(feature = "loom")))]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::thread;
    use std::time::Duration;
    use waitx::*;

    #[test]
    fn test_single_send_recv() {
        let (tx, rx) = channel::<u8>();
        tx.send(42);
        assert_eq!(rx.recv(), 42);
    }

    #[test]
    fn test_multiple_values() {
        let (tx, rx) = channel::<u8>();
        for i in 0..10 {
            tx.send(i);
            assert_eq!(rx.recv(), i);
        }
    }

    #[test]
    fn test_receiver_blocks_until_send() {
        let (tx, rx) = channel::<u8>();
        let handle = thread::spawn(move || rx.recv());
        thread::sleep(Duration::from_millis(50));
        tx.send(99);
        assert_eq!(handle.join().unwrap(), 99);
    }

    #[test]
    fn test_spsc_rapid_fire() {
        let (tx, rx) = channel::<usize>();
        let handle = thread::spawn(move || {
            for i in 0..1000 {
                tx.send(i);
            }
        });
        for i in 0..1000 {
            assert_eq!(rx.recv(), i);
        }
        handle.join().unwrap();
    }

    #[test]
    fn test_random_delays() {
        let (tx, rx) = channel::<usize>();
        let handle = thread::spawn(move || {
            for i in 0..100 {
                thread::sleep(Duration::from_micros(10));
                tx.send(i);
            }
        });
        for i in 0..100 {
            assert_eq!(rx.recv(), i);
        }
        handle.join().unwrap();
    }

    #[test]
    fn test_drop_counter_without_recv() {
        struct DropCounter(Arc<AtomicUsize>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = channel::<DropCounter>();
        tx.send(DropCounter(counter.clone()));
        drop(rx); // drop receiver
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_drop_counter_recv() {
        struct DropCounter(Arc<AtomicUsize>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = channel::<DropCounter>();
        tx.send(DropCounter(counter.clone()));
        rx.recv(); // consume sent value
        drop(rx);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_stress_large_numbers() {
        let (tx, rx) = channel::<usize>();
        let handle = thread::spawn(move || {
            for i in 0..100_000 {
                tx.send(i);
            }
        });
        for i in 0..100_000 {
            assert_eq!(rx.recv(), i);
        }
        handle.join().unwrap();
    }

    #[test]
    fn test_zero_size_type() {
        let (tx, rx) = channel::<()>();
        tx.send(());
        assert_eq!(rx.recv(), ());
    }

    #[test]
    fn test_send_non_copy_type() {
        #[derive(Debug, PartialEq)]
        struct NonCopy(String);
        let (tx, rx) = channel::<NonCopy>();
        tx.send(NonCopy("hello".into()));
        assert_eq!(rx.recv(), NonCopy("hello".into()));
    }

    #[test]
    fn test_try_send_fails_if_full() {
        let (tx, rx) = channel::<u8>();
        tx.send(1);
        assert!(tx.try_send(2).is_err());
        assert_eq!(rx.recv(), 1);
    }

    #[test]
    fn test_spsc_randomized_stress() {
        use rand::Rng;
        use std::thread;
        use std::time::Duration;

        let (tx, rx) = channel::<usize>();
        let rx = std::sync::Arc::new(rx);

        let num_iterations = 10_000;

        let sender = thread::spawn(move || {
            let mut rng = rand::rng();
            for i in 0..num_iterations {
                tx.send(i);
                // Random tiny delay to simulate preemption
                if rng.random_bool(0.05) {
                    thread::sleep(Duration::from_micros(rng.random_range(0..50)));
                }
            }
        });

        let receiver = {
            let rx = rx.clone();
            thread::spawn(move || {
                let mut last = 0;
                for _ in 0..num_iterations {
                    let val = rx.recv();
                    assert!(val >= last); // ensure order is preserved
                    last = val;
                }
            })
        };

        sender.join().unwrap();
        receiver.join().unwrap();
    }
}
