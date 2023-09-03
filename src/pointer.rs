use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};

/// Encapsulate a pointer that need to be free by some mechanism.
pub struct Owned<T> {
    ptr: *mut T,
    dtor: Option<Dtor<T>>,
}

impl<T> Owned<T> {
    /// # Safety
    /// `ptr` must be valid.
    pub unsafe fn new(ptr: *mut T, dtor: Dtor<T>) -> Self {
        Self {
            ptr,
            dtor: Some(dtor),
        }
    }
}

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        match self.dtor.take().unwrap() {
            Dtor::Function(f) => f(self.ptr),
            Dtor::Closure(f) => f(self.ptr),
        }
    }
}

impl<T> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<T> AsRef<T> for Owned<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T> AsMut<T> for Owned<T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

/// A destructor for an object encapsulated by [`Owned`].
pub enum Dtor<T> {
    Function(fn(*mut T)),
    Closure(Box<dyn FnOnce(*mut T)>),
}
