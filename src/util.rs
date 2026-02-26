/// Tuning parameters used to configure the spinning behaviour of [`Waiter`].
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    pub(crate) busy_iters: u32,
    pub(crate) yield_iters: u32,
}

impl Tuning {
    /// Default tuning parameters, with a slight bias towards improved latency.
    pub const DEFAULT: Tuning = Tuning {
        busy_iters: 2_048,
        yield_iters: 256,
    };

    /// Create a custom tuning configuration.
    pub const fn new(busy_iters: u32, yield_iters: u32) -> Self {
        Self {
            busy_iters,
            yield_iters,
        }
    }

    /// Set the maximum number of the initial pure spin phase iterations.
    pub fn busy_iters(mut self, t: u32) -> Self {
        self.busy_iters = t;
        self
    }

    /// Set the maximum number of the spin-yield phase iterations.
    pub fn yield_iters(mut self, t: u32) -> Self {
        self.yield_iters = t;
        self
    }
}

impl Default for Tuning {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(not(feature = "loom"))]
use std::{
    sync::atomic::{AtomicU32, Ordering},
    thread,
};

/// Spins, yields, then blocks via `atomic_wait` until `f` returns `true`.
#[cfg(not(feature = "loom"))]
#[inline]
pub fn wait_until_with_tuning(mut f: impl FnMut() -> bool, wake: &AtomicU32, tuning: Tuning) {
    let Tuning {
        busy_iters,
        yield_iters,
    } = tuning;

    // phase 1: busy spin
    for _ in 0..busy_iters {
        if f() {
            return;
        }
        std::hint::spin_loop();
    }

    // phase 2: yield spin
    for _ in 0..yield_iters {
        if f() {
            return;
        }
        thread::yield_now();
    }

    // phase 3: futex / WaitOnAddress
    loop {
        let val = wake.load(Ordering::Acquire);
        if f() {
            return;
        }
        atomic_wait::wait(wake, val);
        if f() {
            return;
        }
    }
}

#[cfg(not(feature = "loom"))]
#[allow(unused)]
#[inline(always)]
pub fn wait_until(f: impl FnMut() -> bool, wake: &AtomicU32) {
    wait_until_with_tuning(f, wake, Tuning::DEFAULT);
}
