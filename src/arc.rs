use std::ptr::NonNull;

use crate::{heaped_object::GCHeapedObject, traceable::GCTraceable};

#[allow(dead_code)]
pub trait GCRef {
    fn strong_ref(&self) -> usize;
    fn weak_ref(&self) -> usize;
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

    pub fn mark_and_visit(&self) {
        // mark as visited
        unsafe {
            if self.obj.as_ref().is_marked() {
                return;
            }
            self.obj.as_ref().mark();
        }
        self.visit();
    }
    pub fn unmark(&self) {
        // unmark as visited
        unsafe {
            self.obj.as_ref().unmark();
        }
    }

    pub fn downcast<T: GCTraceable + 'static>(self: &Self) -> &T {
        unsafe { self.obj.as_ref().downcast::<T>() }
    }

    pub fn downcast_mut<T: GCTraceable + 'static>(self: &mut Self) -> &mut T {
        unsafe { self.obj.as_mut().downcast_mut::<T>() }
    }

    pub fn isinstance<T: GCTraceable + 'static>(self: &Self) -> bool {
        unsafe { self.obj.as_ref().isinstance::<T>() }
    }

    fn visit(&self) {
        unsafe {
            self.obj.as_ref().as_ref().visit();
        }
    }

    pub(crate) fn ptr_eq(a: &GCArc, b: &GCArc) -> bool {
        unsafe { std::ptr::eq(a.obj.as_ref(), b.obj.as_ref()) }
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
                self.obj.as_mut().drop_value();
                // 如果没有弱引用，释放对象
                if self.obj.as_ref().weak_ref() == 0 {
                    drop(Box::from_raw(self.obj.as_ptr()));
                }
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
            if self
                .obj
                .as_ref()
                .weak_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
            {
                // 如果没有强引用，释放对象
                if self.obj.as_ref().strong_ref() == 0 {
                    drop(Box::from_raw(self.obj.as_ptr()));
                }
            }
        }
    }
}

unsafe impl Send for GCArcWeak {}
unsafe impl Sync for GCArcWeak {}
