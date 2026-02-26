use crate::util::*;

#[cfg(not(feature = "loom"))]
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

#[cfg(feature = "loom")]
use loom::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "loom")]
use loom::sync::{Arc, Condvar, Mutex};

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

/// Sends notifications.
#[derive(Clone)]
pub struct Waker {
    inner: Arc<Inner>,
}

impl Waker {
    /// Wakes the associated [`Waiter`] by incrementing the event counter and waking the waiting thread.
    #[inline(always)]
    pub fn signal(&self) {
        #[cfg(not(feature = "loom"))]
        {
            self.inner.counter.fetch_add(1, Ordering::Release);
            self.inner.wake.fetch_add(1, Ordering::Release);
            atomic_wait::wake_one(&self.inner.wake);
        }

        #[cfg(feature = "loom")]
        {
            *self.inner.counter.lock().unwrap() += 1;
            self.inner.condvar.notify_one();
        }
    }

    /// Wakes the associated [`Waiter`] only if it is currently waiting.
    #[inline(always)]
    pub fn wake(&self) {
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

/// Receives notifications.
pub struct Waiter {
    inner: Arc<Inner>,
    next: AtomicU64,
}

impl Waiter {
    /// Wait for a [`Waker`] signal, using the provided tuning parameters.
    #[inline]
    pub fn wait_with_tuning(&self, tuning: Tuning) {
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

    /// Wait for a [`Waker`] signal, using the default tuning parameters.
    #[inline(always)]
    pub fn wait(&self) {
        self.wait_with_tuning(Tuning::DEFAULT);
    }

    /// Check if a signal is available without blocking.
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

/// Creates a linked [`Waker`] / [`Waiter`] pair.
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
