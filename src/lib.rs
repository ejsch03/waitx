use parking_lot::Mutex;
use spin_sleep::SpinSleeper;
use std::hint::spin_loop;
use std::sync::atomic::AtomicBool;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::thread::{Thread, current};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    busy_spin: Duration,
    fast_spin: Duration,
    fast_chunk: Duration,
}

impl Tuning {
    pub const DEFAULT: Tuning = Tuning {
        busy_spin: Duration::from_micros(8),
        fast_spin: Duration::from_micros(128),
        fast_chunk: Duration::from_micros(32),
    };

    /// Create a tuning configuration with default latency-oriented values.
    pub const fn new(busy_spin: Duration, fast_spin: Duration, fast_chunk: Duration) -> Self {
        Self {
            busy_spin,
            fast_spin,
            fast_chunk,
        }
    }

    /// Set the maximum duration of the initial pure spin phase.
    pub fn busy_spin(mut self, t: Duration) -> Self {
        self.busy_spin = t;
        self
    }

    /// Set the maximum duration of the spin-sleep phase.
    pub fn fast_spin(mut self, t: Duration) -> Self {
        self.fast_spin = t;
        self
    }

    /// Set the sleep duration for each spin-sleep chunk.
    pub fn fast_chunk(mut self, chunk_t: Duration) -> Self {
        self.fast_chunk = chunk_t;
        self
    }
}

impl Default for Tuning {
    fn default() -> Self {
        Self::DEFAULT
    }
}

struct Inner {
    counter: AtomicU64,
    parked: AtomicBool,
    thread: Mutex<Thread>,
}

/// Sends notifications.
#[derive(Clone)]
pub struct UnitSender {
    inner: Arc<Inner>,
}

impl UnitSender {
    pub fn send(&self) {
        self.inner.counter.fetch_add(1, Ordering::Release);

        // only unpark if needed
        if self.inner.parked.load(Ordering::Relaxed) {
            self.inner.thread.lock().unpark();
        }
    }
}

/// Receives notifications.
#[derive(Clone)]
pub struct UnitReceiver {
    inner: Arc<Inner>,
    next: Arc<AtomicU64>,
    spinner: SpinSleeper,
}

impl UnitReceiver {
    /// update the shared [`Thread`] with the current thread.
    pub fn update_thread(&self) {
        *self.inner.thread.lock() = current()
    }

    /// mitigate [`Ordering::Acquire`] spam.
    fn fence(&self, target: u64) -> bool {
        if self.inner.counter.load(Ordering::Relaxed) >= target {
            std::sync::atomic::fence(Ordering::Acquire);
            true
        } else {
            false
        }
    }

    /// recv with explicit tuning (pure spin -> spin-sleep -> park).
    pub fn recv_with_tuning(
        &self,
        Tuning {
            busy_spin,
            fast_spin,
            fast_chunk,
        }: Tuning,
    ) {
        // which event index this recv() is waiting for
        let target = self.next.fetch_add(1, Ordering::Relaxed);

        // phase 1: busy wait
        let pure_start = Instant::now();
        while pure_start.elapsed() < busy_spin {
            if self.fence(target) {
                return;
            }
            spin_loop();
        }

        // phase 2: spin-sleep
        let start_spinsleep = Instant::now();
        while start_spinsleep.elapsed() < fast_spin {
            if self.fence(target) {
                return;
            }
            self.spinner.sleep(fast_chunk);
        }

        // phase 3: park
        loop {
            if self.fence(target) {
                return;
            }

            // mark as parked before re-checking
            self.inner.parked.store(true, Ordering::Relaxed);

            // re-check after marking parked to avoid missing a send
            if self.fence(target) {
                self.inner.parked.store(false, Ordering::Relaxed);
                return;
            }

            // continue with parking
            std::thread::park();
            self.inner.parked.store(false, Ordering::Relaxed);
        }
    }

    /// recv using the default tuning parameters.
    pub fn recv(&self) {
        self.recv_with_tuning(Tuning::DEFAULT);
    }
}

pub fn unit_channel() -> (UnitSender, UnitReceiver) {
    let inner = Arc::new(Inner {
        counter: Default::default(),
        parked: Default::default(),
        thread: Mutex::new(current()),
    });

    let sender = UnitSender {
        inner: inner.clone(),
    };

    let next = Arc::new(AtomicU64::new(1));
    let spinner = SpinSleeper::default();

    let receiver = UnitReceiver {
        inner,
        next,
        spinner,
    };

    (sender, receiver)
}
