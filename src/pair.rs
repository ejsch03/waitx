use crate::util::*;
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::thread::{Thread, current};

struct Inner {
    counter: AtomicU64,
    waiting: AtomicBool,
    thread: Mutex<Thread>,
}

struct WaitingGuard<'a>(&'a AtomicBool);

impl<'a> WaitingGuard<'a> {
    #[inline(always)]
    fn new(flag: &'a AtomicBool) -> Self {
        flag.store(true, Ordering::Release);
        Self(flag)
    }
}

impl<'a> Drop for WaitingGuard<'a> {
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
    /// Wakes the associated [`Waiter`] by incrementing the event counter and unparking the thread.
    ///
    /// Use this when you want accumulating, sequential wakes.
    #[inline(always)]
    pub fn signal(&self) {
        self.inner.counter.fetch_add(1, Ordering::Release);
        self.inner.thread.lock().unpark();
    }

    /// Wakes the associated [`Waiter`] only if it is currently waiting.
    ///
    /// Use this when you want to to wake the waiter only when it's actively waiting.
    #[inline(always)]
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
    #[inline(always)]
    pub fn update_thread(&self) {
        *self.inner.thread.lock() = current()
    }

    /// Wait for [`Waker`], using provided tuning parameters.
    #[inline]
    pub fn wait_with_tuning(&self, tuning: Tuning) {
        let target = self.next.fetch_add(1, Ordering::Relaxed) + 1;

        // if the event already happened, avoid setting waiting flag
        if self.inner.counter.load(Ordering::Acquire) >= target {
            return;
        }

        let _wg = WaitingGuard::new(&self.inner.waiting);

        wait_until_with_tuning(
            || self.inner.counter.load(Ordering::Acquire) >= target,
            tuning,
        );
    }

    /// Wait for [`Waker`], using the default tuning parameters.
    #[inline(always)]
    pub fn wait(&self) {
        self.wait_with_tuning(Tuning::DEFAULT);
    }

    /// Check if notified without blocking.
    #[inline]
    pub fn try_wait(&self) -> bool {
        let target = self.next.load(Ordering::Relaxed) + 1;

        if self.inner.counter.load(Ordering::Acquire) >= target {
            self.next.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
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
