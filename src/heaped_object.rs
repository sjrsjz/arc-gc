use std::{any::TypeId, sync::atomic::{AtomicBool, AtomicUsize}};

use crate::traceable::GCTraceable;


pub struct GCHeapedObject {
    pub value: *mut dyn GCTraceable,
    pub type_id: TypeId,
    pub strong_rc: AtomicUsize,
    pub weak_rc: AtomicUsize,
    pub marked: AtomicBool,
    pub dropped: AtomicBool,
}

#[allow(dead_code)]
impl GCHeapedObject {
    pub fn new<T: GCTraceable + 'static>(value: T) -> Self {
        Self {
            value: Box::into_raw(Box::new(value)),
            type_id: TypeId::of::<T>(),
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

    pub fn downcast_mut<T: GCTraceable + 'static>(self: &mut Self) -> &mut T {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Attempted to access a dropped object");
        }
        if self.type_id != TypeId::of::<T>() {
            panic!("Type mismatch: expected {:?}, found {:?}", TypeId::of::<T>(), self.type_id);
        }
        unsafe { &mut *(self.value as *mut dyn GCTraceable as *mut T) }
    }
    pub fn downcast<T: GCTraceable + 'static>(self: &Self) -> &T {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Attempted to access a dropped object");
        }
        if self.type_id != TypeId::of::<T>() {
            panic!("Type mismatch: expected {:?}, found {:?}", TypeId::of::<T>(), self.type_id);
        }
        unsafe { &*(self.value as *const dyn GCTraceable as *const T) }
    }

    pub(crate) fn drop_value(self: &mut Self) {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        unsafe {
            drop(Box::from_raw(self.value));
            self.dropped.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn isinstance<T: GCTraceable + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub fn as_ref(&self) -> &dyn GCTraceable {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Attempted to access a dropped object");
        }
        unsafe { &*(self.value as *const dyn GCTraceable) }
    }

    pub fn as_mut(&mut self) -> &mut dyn GCTraceable {
        if self.dropped.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Attempted to access a dropped object");
        }
        unsafe { &mut *(self.value as *mut dyn GCTraceable) }
    }
}

impl Drop for GCHeapedObject {
    fn drop(&mut self) {
        self.drop_value();        
    }
}
