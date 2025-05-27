use std::sync::atomic::{AtomicBool, AtomicUsize};

use crate::traceable::GCTraceable;

pub struct GCHeapedObject<T: GCTraceable + 'static> {
    pub value: *mut T,
    pub strong_rc: AtomicUsize,
    pub weak_rc: AtomicUsize,
    pub marked: AtomicBool,
    pub dropped: AtomicBool,
}

#[allow(dead_code)]
impl<T> GCHeapedObject<T>
where
    T: GCTraceable + 'static,
{
    pub fn new(value: T) -> Self {
        Self {
            value: Box::into_raw(Box::new(value)),
            strong_rc: AtomicUsize::new(1),
            weak_rc: AtomicUsize::new(0),
            marked: AtomicBool::new(false),
            dropped: AtomicBool::new(false),
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

    pub(crate) fn drop_value(self: &mut Self) {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        unsafe {
            drop(Box::from_raw(self.value));
            self.dropped
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn as_ref(&self) -> &T {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Attempted to access a dropped object");
        }
        unsafe { &*self.value }
    }

    pub fn as_mut(&mut self) -> &mut T {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Attempted to access a dropped object");
        }
        unsafe { &mut *self.value }
    }
}

impl<T> Drop for GCHeapedObject<T>
where
    T: GCTraceable + 'static,
{
    fn drop(&mut self) {
        self.drop_value();
    }
}
