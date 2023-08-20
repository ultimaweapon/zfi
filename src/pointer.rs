use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};

/// Encapsulate a pointer that need to be free by some mechanism.
pub struct Owned<T> {
    ptr: *mut T,
    dtor: Option<Box<dyn FnOnce(*mut T)>>,
}

impl<T> Owned<T> {
    /// # Safety
    /// `ptr` must be valid.
    pub unsafe fn new<D>(ptr: *mut T, dtor: D) -> Self
    where
        D: FnOnce(*mut T) + 'static,
    {
        Self {
            ptr,
            dtor: Some(Box::new(dtor)),
        }
    }
}

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        self.dtor.take().unwrap()(self.ptr);
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
        &self
    }
}

impl<T> AsMut<T> for Owned<T> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}
