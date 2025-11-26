use parking_lot::Mutex;
use std::hint::spin_loop;
use std::sync::atomic::AtomicBool;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::thread::{Thread, current, yield_now};

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

struct Inner {
    counter: AtomicU64,
    waiting: AtomicBool,
    thread: Mutex<Thread>,
}

struct WaitingGuard<'a>(&'a AtomicBool);

impl<'a> WaitingGuard<'a> {
    #[inline]
    fn new(flag: &'a AtomicBool) -> Self {
        flag.store(true, Ordering::Release);
        Self(flag)
    }
}

impl<'a> Drop for WaitingGuard<'a> {
    #[inline]
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn wait(counter: &AtomicU64, target: u64, tuning: Tuning) {
    let Tuning {
        busy_iters,
        yield_iters,
    } = tuning;
    // phase 1: busy spin using Relaxed loads
    for _ in 0..busy_iters {
        // re-check with Acquire before returning.
        if counter.load(Ordering::Relaxed) >= target {
            std::sync::atomic::fence(Ordering::Acquire);
            return;
        }
        spin_loop();
    }

    // phase 2: yield spin
    for _ in 0..yield_iters {
        if counter.load(Ordering::Relaxed) >= target {
            std::sync::atomic::fence(Ordering::Acquire);
            return;
        }
        yield_now();
    }

    // phase 3: park
    loop {
        if counter.load(Ordering::Acquire) >= target {
            break;
        }
        std::thread::park();
    }
}

/// Sends notifications.
#[derive(Clone)]
pub struct Waker {
    inner: Arc<Inner>,
}

impl Waker {
    /// Wakes the associated [`Waiter`] by incrementing the event counter and unparking the thread.
    ///
    /// Use this when you want accumulating, sequential wakes.
    #[inline]
    pub fn signal(&self) {
        self.inner.counter.fetch_add(1, Ordering::Release);
        self.inner.thread.lock().unpark();
    }

    /// Wakes the associated [`Waiter`] only if it is currently waiting.
    ///
    /// Use this when you want to to wake the waiter only when it's actively waiting.
    #[inline]
    pub fn wake(&self) {
        if self.inner.waiting.load(Ordering::Acquire) {
            self.signal();
        }
    }
}

/// Receives notifications.
pub struct Waiter {
    inner: Arc<Inner>,
    next: Arc<AtomicU64>,
}

impl Waiter {
    /// Marks the current thread as the target for wake operations.
    ///
    /// Should be called once by the thread that will be waiting.
    #[inline]
    pub fn update_thread(&self) {
        *self.inner.thread.lock() = current()
    }

    /// Wait for [`Waker`], using provided tuning parameters.
    pub fn wait_with_tuning(&self, tuning: Tuning) {
        let target = self.next.fetch_add(1, Ordering::Relaxed) + 1;

        // if the event already happened, avoid setting waiting flag
        if self.inner.counter.load(Ordering::Acquire) >= target {
            return;
        }

        let _wg = WaitingGuard::new(&self.inner.waiting);
        wait(&self.inner.counter, target, tuning);
    }

    /// Wait for [`Waker`], using the default tuning parameters.
    #[inline]
    pub fn wait(&self) {
        self.wait_with_tuning(Tuning::DEFAULT);
    }
}

/// Creates a new synchronous, empty channel.
pub fn pair() -> (Waker, Waiter) {
    let inner = Arc::new(Inner {
        counter: Default::default(),
        waiting: Default::default(),
        thread: Mutex::new(current()),
    });
    let waker = Waker {
        inner: inner.clone(),
    };
    let waiter = Waiter {
        inner,
        next: Default::default(),
    };
    (waker, waiter)
}
