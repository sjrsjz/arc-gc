use std::{
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicUsize},
};

pub trait GCTraceable {
    fn visit(&self) {}
}

pub struct GCHeapedObject {
    pub value: Box<dyn GCTraceable>,
    pub strong_rc: AtomicUsize,
    pub weak_rc: AtomicUsize,
    pub marked: AtomicBool,
}

impl GCHeapedObject {
    pub fn new<T: GCTraceable + 'static>(value: T) -> Self {
        Self {
            value: Box::new(value),
            strong_rc: AtomicUsize::new(1),
            weak_rc: AtomicUsize::new(0),
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

    pub fn downcast_mut<T: GCTraceable>(self: &mut Self) -> &mut T {
        unsafe { &mut *(self.value.as_mut() as *mut dyn GCTraceable as *mut T) }
    }
    pub fn downcast<T: GCTraceable>(self: &Self) -> &T {
        unsafe { &*(self.value.as_ref() as *const dyn GCTraceable as *const T) }
    }
}

#[allow(dead_code)]
pub trait GCRef {
    fn strong_ref(&self) -> usize;
    fn weak_ref(&self) -> usize;
    fn mark_and_visit(&self) {
        let obj: NonNull<GCHeapedObject> = self.obj_ref();
        // mark as visited
        unsafe {
            if obj.as_ref().is_marked() {
                return;
            }
            obj.as_ref().mark();
        }
        self.visit();
    }
    fn unmark(&self) {
        let obj = self.obj_ref();
        // unmark as visited
        unsafe {
            obj.as_ref().unmark();
        }
    }
    fn visit(&self) {}
    fn obj_ref(&self) -> NonNull<GCHeapedObject>;
    fn downcast<T: GCTraceable>(self: &Self) -> &T {
        unsafe { self.obj_ref().as_ref().downcast::<T>() }
    }

    fn downcast_mut<T: GCTraceable>(self: &mut Self) -> &mut T {
        unsafe { self.obj_ref().as_mut().downcast_mut::<T>() }
    }
}

pub struct GCArc {
    obj: NonNull<GCHeapedObject>,
}

#[allow(dead_code)]
impl GCArc {
    pub fn new<T: GCTraceable + 'static>(obj: T) -> Self {
        let heaped_obj = Box::new(GCHeapedObject::new(obj));
        let obj_ptr = Box::into_raw(heaped_obj);
        Self {
            obj: NonNull::new(obj_ptr).expect("Unable to create GCArc"),
        }
    }
    pub unsafe fn inc_ref(&self) {
        unsafe {
            self.obj
                .as_ref()
                .strong_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub unsafe fn dec_ref(&self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .strong_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to decrement a GCArc with 0 strong references");
            }
            if self
                .obj
                .as_ref()
                .strong_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
            {
                drop(Box::from_raw(self.obj.as_ptr()));
            }
        }
    }

    pub fn as_weak(&self) -> GCArcWeak {
        unsafe {
            self.obj
                .as_ref()
                .weak_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        GCArcWeak { obj: self.obj }
    }

    pub fn is_marked(&self) -> bool {
        unsafe { self.obj.as_ref().is_marked() }
    }
}

impl Clone for GCArc {
    fn clone(&self) -> Self {
        unsafe {
            self.obj
                .as_ref()
                .strong_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        Self { obj: self.obj }
    }
}

impl GCRef for GCArc {
    fn strong_ref(&self) -> usize {
        unsafe { self.obj.as_ref().strong_ref() }
    }

    fn weak_ref(&self) -> usize {
        unsafe { self.obj.as_ref().weak_ref() }
    }

    fn visit(&self) {
        unsafe {
            self.obj.as_ref().value.visit();
        }
    }

    fn obj_ref(&self) -> NonNull<GCHeapedObject> {
        self.obj
    }
}

impl Drop for GCArc {
    fn drop(&mut self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .strong_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to drop a GCArc with 0 strong references");
            }
            if self
                .obj
                .as_mut()
                .strong_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
            {
                drop(Box::from_raw(self.obj.as_ptr()));
            }
        }
    }
}

unsafe impl Send for GCArc {}
unsafe impl Sync for GCArc {}

pub struct GCArcWeak {
    obj: NonNull<GCHeapedObject>,
}

#[allow(dead_code)]
impl GCArcWeak {
    pub unsafe fn from_raw(obj: NonNull<GCHeapedObject>) -> Self {
        Self { obj }
    }
    pub(crate) fn is_marked(&self) -> bool {
        unsafe { self.obj.as_ref().is_marked() }
    }

    pub fn upgrade(&self) -> Option<GCArc> {
        unsafe {
            let strong_count = self
                .obj
                .as_ref()
                .strong_rc
                .load(std::sync::atomic::Ordering::SeqCst);
            if strong_count == 0 {
                // 对象已被释放，无法升级
                return None;
            }

            // 增加强引用计数
            self.obj
                .as_ref()
                .strong_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(GCArc { obj: self.obj })
        }
    }
}

impl Clone for GCArcWeak {
    fn clone(&self) -> Self {
        unsafe {
            self.obj
                .as_ref()
                .weak_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        Self { obj: self.obj }
    }
}

impl GCRef for GCArcWeak {
    fn strong_ref(&self) -> usize {
        unsafe { self.obj.as_ref().strong_ref() }
    }

    fn weak_ref(&self) -> usize {
        unsafe { self.obj.as_ref().weak_ref() }
    }

    fn visit(&self) {
        unsafe { self.obj.as_ref().value.visit() }
    }

    fn obj_ref(&self) -> NonNull<GCHeapedObject> {
        self.obj
    }
}

impl Drop for GCArcWeak {
    fn drop(&mut self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .weak_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to drop a GCArcWeak with 0 weak references");
            }
            self.obj
                .as_ref()
                .weak_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

unsafe impl Send for GCArcWeak {}
unsafe impl Sync for GCArcWeak {}
