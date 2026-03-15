//! Cross platform atomic wait and wake (aka futex) functionality.
//!
//! This module only supports functionality that's available on all of
//! Linux, FreeBSD, Windows, and macOS. That is:
//!
//! - Only `AtomicU32` is supported.
//!   (Linux currently only supports 32-bit futexes.)
//! - Only the "wait", "wake one", and "wake all" operations are supported.
//!   (Linux supports more operations, but Windows and macOS don't.)
//! - No timeouts.
//!   (macOS doesn't have a stable/public API for timeouts.)
//! - The wake operations don't return the number of threads woken up.
//!   (Only Linux supports this.)
//!
//! Supported platforms:
//!    Linux 2.6.22+,
//!    FreeBSD 11+,
//!    Windows 8+, Windows Server 2012+,
//!    macOS 11+, iOS 14+, watchOS 7+.
//!
//! ## Usage
//!
//! ```rust
//! use std::sync::atomic::AtomicU32;
//!
//! let a = AtomicU32::new(0);
//!
//! atomic_wait::wait(&a, 1); // If the value is 1, wait.
//!
//! atomic_wait::wake_one(&a); // Wake one waiting thread.
//! ```
//!
//! ## Implementation
//!
//! On Linux, this uses the `SYS_futex` syscall.
//!
//! On FreeBSD, this uses the `_umtx_op` syscall.
//!
//! On Windows, this uses the `WaitOnAddress` and `WakeByAddress` APIs.
//!
//! On macOS (and iOS and watchOS), this uses `libc++`, making use of the same
//! (ABI-stable) functions behind C++20's `atomic_wait` and `atomic_notify` functions.
//!
//! ----------------------------------------------------------------------------------
//!
//! Vendored from <https://github.com/m-ou-se/atomic-wait>
//! License: BSD-2-Clause (see LICENSES/LICENSE-atomic-wait)

use core::sync::atomic::AtomicU32;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod platform;

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "watchos"))]
#[path = "macos.rs"]
mod platform;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[cfg(target_os = "freebsd")]
#[path = "freebsd.rs"]
mod platform;

/// If the value is `value`, wait until woken up.
///
/// This function might also return spuriously,
/// without a corresponding wake operation.
#[inline]
pub fn wait(atomic: &AtomicU32, value: u32) {
    platform::wait(atomic, value)
}

/// Wake one thread that is waiting on this atomic.
///
/// It's okay if the pointer dangles or is null.
#[inline]
pub fn wake_one(atomic: *const AtomicU32) {
    platform::wake_one(atomic);
}
