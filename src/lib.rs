use parking_lot::Mutex;
use std::hint::spin_loop;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::thread::{Thread, current, yield_now};

/// Tuning parameters used to configure the spinning times of [`UnitReceiver`].
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
    thread: Mutex<Thread>,
}

/// Sends notifications.
#[derive(Clone)]
pub struct UnitSender {
    inner: Arc<Inner>,
}

impl UnitSender {
    #[inline]
    pub fn send(&self) {
        self.inner.counter.fetch_add(1, Ordering::Release);
        self.inner.thread.lock().unpark();
    }
}

/// Receives notifications.
pub struct UnitReceiver {
    inner: Arc<Inner>,
    next: Arc<AtomicU64>,
}

impl UnitReceiver {
    /// Set the shared [`Thread`] to the current thread.
    #[inline]
    pub fn update_thread(&self) {
        *self.inner.thread.lock() = current()
    }

    /// Wait for [`UnitSender`], using provided tuning parameters.
    pub fn recv_with_tuning(
        &self,
        Tuning {
            busy_iters,
            yield_iters,
        }: Tuning,
    ) {
        // which event index this recv() is waiting for
        let target = self.next.fetch_add(1, Ordering::Relaxed) + 1;

        // phase 1: busy wait
        if busy_iters != 0 {
            let mut i = 0;
            while i < busy_iters {
                if self.reached(target) {
                    return;
                }
                spin_loop();
                i += 1;
            }
        }

        // phase 2: yield wait
        if yield_iters != 0 {
            let mut i = 0;
            while i < yield_iters {
                if self.reached(target) {
                    return;
                }
                yield_now();
                i += 1;
            }
        }

        // phase 3: park
        loop {
            if self.reached(target) {
                break;
            }
            std::thread::park();
        }
    }

    /// Wait for [`UnitSender`], using the default tuning parameters.
    #[inline]
    pub fn recv(&self) {
        self.recv_with_tuning(Tuning::DEFAULT);
    }

    #[inline(always)]
    fn reached(&self, target: u64) -> bool {
        self.inner.counter.load(Ordering::Acquire) >= target
    }
}

/// Creates a new synchronous, empty channel.
pub fn unit_channel() -> (UnitSender, UnitReceiver) {
    let inner = Arc::new(Inner {
        counter: Default::default(),
        thread: Mutex::new(current()),
    });

    let sender = UnitSender {
        inner: inner.clone(),
    };

    let receiver = UnitReceiver {
        inner,
        next: Default::default(),
    };

    (sender, receiver)
}
