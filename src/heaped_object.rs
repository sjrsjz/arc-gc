use std::{ptr::NonNull, sync::atomic::{AtomicBool, AtomicUsize}};

use crate::traceable::GCTraceable;

pub struct GCHeapedObject<T: GCTraceable<T> + 'static> {
    value: Option<NonNull<T>>,
    pub(crate) strong_rc: AtomicUsize,
    pub(crate) weak_rc: AtomicUsize,
    pub(crate) marked: AtomicBool,
}

#[allow(dead_code)]
impl<T> GCHeapedObject<T>
where
    T: GCTraceable<T> + 'static,
{
    pub fn new(value: T) -> Self {
        Self {
            value: NonNull::new(Box::into_raw(Box::new(value))),
            strong_rc: AtomicUsize::new(1),
            weak_rc: AtomicUsize::new(1),
            marked: AtomicBool::new(false),
        }
    }

    pub fn strong_ref(&self) -> usize {
        self.strong_rc.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn weak_ref(&self) -> usize {
        self.weak_rc.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn mark(&self) {
        self.marked.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn unmark(&self) {
        self.marked
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_marked(&self) -> bool {
        self.marked.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_dropped(&self) -> bool {
        self.value.is_none()
    }

    pub(crate) fn drop_value(self: &mut Self) {
        if let Some(ptr) = self.value {
            self.value = None;
            unsafe {
                drop(Box::from_raw(ptr.as_ptr()));
            }
        }
    }

    pub fn as_ref(&self) -> &T {
        match self.value {
            Some(ptr) => {
                unsafe { &*ptr.as_ptr() }
            }
            None => panic!("Attempted to access a value that has been dropped"),            
        }        
    }

    pub fn as_mut(&mut self) -> &mut T {
        match self.value {
            Some(ptr) => {
                unsafe { &mut *ptr.as_ptr() }
            }
            None => panic!("Attempted to access a value that has been dropped"),
        }
    }
}

