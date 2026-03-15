//! A counted, blocking notification primitive.
//!
//! This module provides a [`Waker`]/[`Waiter`] pair where each call to
//! [`Waker::signal`] increments an internal counter and wakes a blocked
//! [`Waiter`]. Notifications are not lost.
//!
//! # Example
//!
//! ```
//! let (waker, waiter) = waitx::pair();
//!
//! std::thread::spawn(move || {
//!     waker.signal();
//! });
//!
//! waiter.wait(); // blocks until signaled
//! ```

use crate::prelude::*;

#[cfg(feature = "loom")]
struct Inner {
    counter: Mutex<u64>,
    condvar: Condvar,
}

#[cfg(not(feature = "loom"))]
struct Inner {
    counter: AtomicU64,
    wake: AtomicU32,
    waiting: AtomicBool,
}

#[cfg(not(feature = "loom"))]
struct WaitingGuard<'a>(&'a AtomicBool);

#[cfg(not(feature = "loom"))]
impl<'a> WaitingGuard<'a> {
    #[inline(always)]
    fn new(flag: &'a AtomicBool) -> Self {
        flag.store(true, Ordering::Release);
        Self(flag)
    }
}

#[cfg(not(feature = "loom"))]
impl Drop for WaitingGuard<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Sends counted notifications to a paired [`Waiter`].
#[derive(Clone)]
pub struct Waker {
    inner: Arc<Inner>,
}

impl Waker {
    /// Increments the event counter and wakes the waiting thread.
    #[inline(always)]
    pub fn signal(&self) {
        #[cfg(not(feature = "loom"))]
        {
            self.inner.counter.fetch_add(1, Ordering::Release);
            self.inner.wake.fetch_add(1, Ordering::Release);
            crate::atomic_wait::wake_one(&self.inner.wake);
        }

        #[cfg(feature = "loom")]
        {
            *self.inner.counter.lock().unwrap() += 1;
            self.inner.condvar.notify_one();
        }
    }

    /// Wakes the waiter only if it is currently blocked.
    #[inline(always)]
    pub fn poke(&self) {
        #[cfg(not(feature = "loom"))]
        {
            if self.inner.waiting.load(Ordering::Acquire) {
                self.signal();
            }
        }

        #[cfg(feature = "loom")]
        self.signal();
    }
}

/// A counted, blocking notification primitive.
pub struct Waiter {
    inner: Arc<Inner>,
    next: AtomicU64,
}

impl Waiter {
    /// Blocks until the next notification, using provided tuning.
    #[inline]
    pub fn wait_with(&self, tuning: Tuning) {
        let target = self.next.fetch_add(1, Ordering::Relaxed) + 1;

        #[cfg(not(feature = "loom"))]
        {
            if self.inner.counter.load(Ordering::Acquire) >= target {
                return;
            }
            let _wg = WaitingGuard::new(&self.inner.waiting);
            wait_until_with_tuning(
                || self.inner.counter.load(Ordering::Acquire) >= target,
                &self.inner.wake,
                tuning,
            );
        }

        #[cfg(feature = "loom")]
        {
            let _ = tuning;
            let mut guard = self.inner.counter.lock().unwrap();
            while *guard < target {
                guard = self.inner.condvar.wait(guard).unwrap();
            }
        }
    }

    /// Blocks until the next notification using default tuning.
    #[inline(always)]
    pub fn wait(&self) {
        self.wait_with(Tuning::DEFAULT);
    }

    /// Attempts to consume a notification without blocking.
    #[inline]
    pub fn try_wait(&self) -> bool {
        let target = self.next.load(Ordering::Relaxed) + 1;

        #[cfg(not(feature = "loom"))]
        let ready = self.inner.counter.load(Ordering::Acquire) >= target;

        #[cfg(feature = "loom")]
        let ready = *self.inner.counter.lock().unwrap() >= target;

        if ready {
            self.next.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

/// Creates a new counted notification pair.
pub fn pair() -> (Waker, Waiter) {
    #[cfg(not(feature = "loom"))]
    let inner = Arc::new(Inner {
        counter: Default::default(),
        wake: Default::default(),
        waiting: Default::default(),
    });

    #[cfg(feature = "loom")]
    let inner = Arc::new(Inner {
        counter: Mutex::new(0),
        condvar: Condvar::new(),
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
