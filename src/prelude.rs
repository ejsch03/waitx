#![allow(unused_imports)]

pub use std::cell::UnsafeCell;
pub use std::mem::MaybeUninit;

#[cfg(feature = "loom")]
pub use loom::{
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    sync::{Arc, Condvar, Mutex},
    thread,
};

#[cfg(not(feature = "loom"))]
pub use std::{
    sync::Arc,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    thread,
};

pub use crate::channel::*;
pub use crate::pair::*;
pub use crate::util::*;
