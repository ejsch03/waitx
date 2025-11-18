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
    const DEFAULT_TUNING: Tuning = Tuning {
        busy_spin: Duration::from_micros(8),
        fast_spin: Duration::from_micros(128),
        fast_chunk: Duration::from_micros(32),
    };

    /// Create a tuning configuration with default latency-oriented values.
    pub const fn new() -> Self {
        Self::DEFAULT_TUNING
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
        Self::new()
    }
}

/// Sends notifications.
#[derive(Clone)]
pub struct UnitSender {
    inner: Arc<AtomicU64>,
    thread: Arc<Mutex<Thread>>,
    parked: Arc<AtomicBool>,
}

impl UnitSender {
    pub fn send(&self) {
        self.inner.fetch_add(1, Ordering::Release);
        // only unpark if needed
        if self.parked.load(Ordering::Relaxed) {
            self.thread.lock().unpark();
        }
    }
}

/// Receives notifications.
#[derive(Clone)]
pub struct UnitReceiver {
    inner: Arc<AtomicU64>,
    next_ticket: Arc<AtomicU64>,
    sleeper: SpinSleeper,
    thread: Arc<Mutex<Thread>>,
    parked: Arc<AtomicBool>,
}

impl UnitReceiver {
    pub fn update_thread(&self) {
        *self.thread.lock() = current()
    }

    /// mitigate [`Ordering::Acquire`] spam
    fn fence(&self, target: u64) -> bool {
        if self.inner.load(Ordering::Relaxed) >= target {
            std::sync::atomic::fence(Ordering::Acquire);
            true
        } else {
            false
        }
    }

    /// recv with explicit tuning (pure spin -> spin-sleep -> park).
    pub fn recv_with_tuning(&self, tuning: Tuning) {
        // which event index this recv() is waiting for
        let target = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        // phase 0: pure spin
        let pure_start = Instant::now();
        while pure_start.elapsed() < tuning.busy_spin {
            if self.fence(target) {
                return;
            }
            spin_loop();
        }

        // phase 1: spin-sleep
        let start_spinsleep = Instant::now();
        while start_spinsleep.elapsed() < tuning.fast_spin {
            if self.fence(target) {
                return;
            }
            self.sleeper.sleep(tuning.fast_chunk);
        }

        // phase 2: park
        loop {
            if self.fence(target) {
                return;
            }

            // mark as parked before re-checking
            self.parked.store(true, Ordering::Relaxed);

            // re-check after marking parked to avoid missing a send
            if self.fence(target) {
                self.parked.store(false, Ordering::Relaxed);
                return;
            }

            // continue with parking
            std::thread::park();
            self.parked.store(false, Ordering::Relaxed);
        }
    }

    /// recv using the default tuning parameters.
    pub fn recv(&self) {
        self.recv_with_tuning(Tuning::DEFAULT_TUNING);
    }
}

pub fn unit_channel() -> (UnitSender, UnitReceiver) {
    let inner: Arc<AtomicU64> = Default::default();
    let thread = Arc::new(Mutex::new(current()));
    let parked: Arc<AtomicBool> = Default::default();

    let sender = UnitSender {
        inner: inner.clone(),
        thread: thread.clone(),
        parked: parked.clone(),
    };

    let receiver = UnitReceiver {
        inner,
        next_ticket: Arc::new(AtomicU64::new(1)),
        sleeper: SpinSleeper::default(),
        thread,
        parked,
    };

    (sender, receiver)
}
