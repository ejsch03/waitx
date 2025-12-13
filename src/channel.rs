use crate::pair::*;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct Slot<T> {
    inner: UnsafeCell<MaybeUninit<T>>,
    full: AtomicBool,
}

impl<T> Slot<T> {
    /// Mark the slot as full
    #[inline(always)]
    pub fn mark_full(&self) {
        self.full.store(true, Ordering::Release);
    }

    /// Mark the slot as empty
    #[inline(always)]
    pub fn mark_empty(&self) {
        self.full.store(false, Ordering::Release);
    }

    /// Check if the slot is full
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

pub struct Sender<T>(Inner<T>);

impl<T> Sender<T> {
    #[inline(always)]
    pub fn update_thread(&self) {
        self.0.rx.update_thread();
    }

    #[inline]
    pub fn send(&self, value: T) {
        // wait until the slot is empty
        self.0.rx.wait();

        // write the value safely
        unsafe {
            (*self.0.slot.inner.get()).write(value);
        }

        // mark slot as full
        self.0.slot.mark_full();

        // notify receiver
        self.0.tx.signal();
    }

    #[inline(always)]
    pub fn try_send(&self, value: T) -> Result<(), T> {
        // exit early if already full
        if !self.0.rx.try_wait() {
            return Err(value);
        }
        unsafe {
            (*self.0.slot.inner.get()).write(value);
        }
        // mark full then notify
        self.0.slot.mark_full();
        self.0.tx.wake();
        Ok(())
    }
}

pub struct Receiver<T>(Inner<T>);

impl<T> Receiver<T> {
    #[inline(always)]
    pub fn update_thread(&self) {
        self.0.rx.update_thread();
    }

    #[inline(always)]
    pub fn recv(&self) -> T {
        self.0.rx.wait();
        self.get()
    }

    #[inline(always)]
    pub fn try_recv(&self) -> Option<T> {
        if !self.0.rx.try_wait() {
            return None;
        }
        Some(self.get())
    }

    #[inline(always)]
    fn get(&self) -> T {
        // SAFETY: slot must be full
        let value = unsafe { (*self.0.slot.inner.get()).assume_init_read() };

        // mark empty then notify
        self.0.slot.mark_empty();
        self.0.tx.signal();

        value
    }
}

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
    rx.0.tx.signal(); // initialize sender
    (tx, rx)
}
