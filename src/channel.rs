//! A minimal synchronous single-slot channel.
//!
//! This module provides a blocking [`Sender`]/[`Receiver`] pair that transfers
//! values through a single shared slot. Sending blocks until the slot is
//! empty; receiving blocks until it is full.
//!
//! # Example
//!
//! ```
//! let (tx, rx) = waitx::channel();
//!
//! std::thread::spawn(move || {
//!     tx.send(42);
//! });
//!
//! assert_eq!(rx.recv(), 42);
//! ```

use crate::prelude::*;

struct Slot<T> {
    inner: UnsafeCell<MaybeUninit<T>>,
    full: AtomicBool,
}

impl<T> Slot<T> {
    #[inline(always)]
    pub fn mark_full(&self) {
        self.full.store(true, Ordering::Release);
    }

    #[inline(always)]
    pub fn mark_empty(&self) {
        self.full.store(false, Ordering::Release);
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.full.load(Ordering::Acquire)
    }
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            inner: UnsafeCell::new(MaybeUninit::uninit()),
            full: AtomicBool::new(false),
        }
    }
}

impl<T> Drop for Slot<T> {
    fn drop(&mut self) {
        if self.is_full() {
            // SAFETY: the value has to exist at this point.
            unsafe {
                (*self.inner.get()).assume_init_drop();
            }
        }
    }
}

unsafe impl<T: Send> Send for Slot<T> {}
unsafe impl<T: Send> Sync for Slot<T> {}

struct Inner<T> {
    slot: Arc<Slot<T>>,
    tx: Waker,
    rx: Waiter,
}

/// Sending half of a single-slot synchronous channel.
pub struct Sender<T>(Inner<T>);

impl<T> Sender<T> {
    /// Sends a value, blocking indefinitely until the slot becomes empty.
    #[inline]
    pub fn send(&self, value: T) {
        // wait until the slot is empty
        self.0.rx.wait();

        // write the value
        unsafe {
            (*self.0.slot.inner.get()).write(value);
        }

        // mark slot as full
        self.0.slot.mark_full();

        // notify receiver
        self.0.tx.signal();
    }

    /// Attempts to send a value without blocking, returning it if the slot is full.
    #[inline(always)]
    pub fn try_send(&self, value: T) -> Result<(), T> {
        // exit early if already full
        if !self.0.rx.try_wait() {
            return Err(value);
        }
        unsafe {
            (*self.0.slot.inner.get()).write(value);
        }
        self.0.slot.mark_full();
        self.0.tx.signal();
        Ok(())
    }
}

/// Receiving half of a single-slot synchronous channel.
pub struct Receiver<T>(Inner<T>);

impl<T> Receiver<T> {
    /// Receives a value, blocking until one is available.
    #[inline(always)]
    pub fn recv(&self) -> T {
        self.0.rx.wait();
        self.get()
    }

    /// Attempts to receive a value without blocking.
    #[inline(always)]
    pub fn try_recv(&self) -> Option<T> {
        if !self.0.rx.try_wait() {
            return None;
        }
        Some(self.get())
    }

    /// Reads and removes the current value from the slot.
    #[inline(always)]
    fn get(&self) -> T {
        // SAFETY: slot must be full at this point.
        let value = unsafe { (*self.0.slot.inner.get()).assume_init_read() };

        self.0.slot.mark_empty();
        self.0.tx.signal();

        value
    }
}

/// Creates a new single-slot synchronous channel.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (tx_1, rx_1) = pair();
    let (tx_2, rx_2) = pair();
    let slot_tx = Arc::new(Slot::default());
    let slot_rx = slot_tx.clone();

    let inner_tx = Inner {
        slot: slot_tx,
        tx: tx_1,
        rx: rx_2,
    };
    let inner_rx = Inner {
        slot: slot_rx,
        tx: tx_2,
        rx: rx_1,
    };

    let (tx, rx) = (Sender(inner_tx), Receiver(inner_rx));
    rx.0.tx.signal(); // initialize sender: slot starts empty
    (tx, rx)
}
