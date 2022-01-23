//! TODO: find a crate with some no_std sync impl
use core::cell::UnsafeCell;

/// Any object implementing this trait guarantees exclusive access to the data contained within
/// the Mutex for the duration of the provided closure.
pub trait MutexTrait {
    /// The type of encapsulated data.
    type Data;

    /// Creates a critical section and grants temporary mutable access to the encapsulated data.
    fn lock<R>(&mut self, f: impl FnOnce(&mut Self::Data) -> R) -> R;
}

/// A pseudo-lock for teaching purposes.
///
/// Used to introduce [interior mutability].
///
/// The lock will only be used as long as it is safe to do so, i.e. as long as the kernel is
/// executing single-threaded, aka only running on a single core with interrupts disabled.
///
/// [interior mutability]: https://doc.rust-lang.org/std/cell/index.html
pub struct NullLock<T: ?Sized> {
    data: UnsafeCell<T>,
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

unsafe impl<T: ?Sized> Sync for NullLock<T> {}

impl<T> NullLock<T> {
    /// Wraps `data` into a new `NullLock`.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------

impl<T> MutexTrait for &NullLock<T> {
    type Data = T;

    fn lock<R>(&mut self, f: impl FnOnce(&mut Self::Data) -> R) -> R {
        // In a real lock, there would be code encapsulating this line that ensures that this
        // mutable reference will ever only be given out once at a time.
        let data = unsafe { &mut *self.data.get() };

        f(data)
    }
}
