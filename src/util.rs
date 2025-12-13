/// Tuning parameters used to configure the spinning times of [`Waiter`].
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    busy_iters: u32,
    yield_iters: u32,
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

    /// Set the maximum u32 of the initial pure spin phase.
    pub fn busy_iters(mut self, t: u32) -> Self {
        self.busy_iters = t;
        self
    }

    /// Set the maximum u32 of the spin-sleep phase.
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

#[inline]
pub fn wait_until_with_tuning(mut f: impl FnMut() -> bool, tuning: Tuning) {
    let Tuning {
        busy_iters,
        yield_iters,
    } = tuning;
    // phase 1: busy spin using Relaxed loads
    for _ in 0..busy_iters {
        // re-check with Acquire before returning.
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
        std::thread::yield_now();
    }

    // phase 3: park
    loop {
        std::thread::park();
        if f() {
            break;
        }
    }
}

#[allow(unused)]
#[inline(always)]
pub fn wait_until(f: impl FnMut() -> bool) {
    wait_until_with_tuning(f, Tuning::DEFAULT);
}
