use std::ptr::NonNull;

use crate::{heaped_object::GCHeapedObject, traceable::GCTraceable};

#[allow(dead_code)]
pub trait GCRef {
    fn strong_ref(&self) -> usize;
    fn weak_ref(&self) -> usize;
    fn inc_ref(&self);
    fn dec_ref(&self);
    fn inc_weak_ref(&self);
    fn dec_weak_ref(&self);
}

pub struct GCArc<T: GCTraceable + 'static> {
    obj: NonNull<GCHeapedObject<T>>,
}

#[allow(dead_code)]
impl<T> GCArc<T>
where
    T: GCTraceable + 'static,
{
    pub fn new(obj: T) -> Self {
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

    pub fn as_weak(&self) -> GCArcWeak<T> {
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

    pub fn as_ref(&self) -> &T {
        unsafe { self.obj.as_ref().as_ref() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.obj.as_mut().as_mut() }
    }

    fn visit(&self) {
        unsafe {
            self.obj.as_ref().as_ref().visit();
        }
    }

    pub(crate) fn ptr_eq(a: &GCArc<T>, b: &GCArc<T>) -> bool {
        unsafe { std::ptr::eq(a.obj.as_ref(), b.obj.as_ref()) }
    }
}

impl<T> Clone for GCArc<T>
where
    T: GCTraceable + 'static,
{
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

impl<T> GCRef for GCArc<T>
where
    T: GCTraceable + 'static,
{
    fn strong_ref(&self) -> usize {
        unsafe { self.obj.as_ref().strong_ref() }
    }

    fn weak_ref(&self) -> usize {
        unsafe { self.obj.as_ref().weak_ref() }
    }

    fn inc_ref(&self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .strong_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to increment a GCArc with 0 strong references");
            }
            self.obj
                .as_ref()
                .strong_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn dec_ref(&self) {
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

    fn inc_weak_ref(&self) {
        unsafe {
            self.obj
                .as_ref()
                .weak_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn dec_weak_ref(&self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .weak_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to decrement a GCArc with 0 weak references");
            }
            if self
                .obj
                .as_ref()
                .weak_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
            {
                drop(Box::from_raw(self.obj.as_ptr()));
            }
        }
    }
}

impl<T> Drop for GCArc<T>
where
    T: GCTraceable + 'static,
{
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

unsafe impl<T> Send for GCArc<T> where T: GCTraceable + 'static {}
unsafe impl<T> Sync for GCArc<T> where T: GCTraceable + 'static {}

pub struct GCArcWeak<T: GCTraceable + 'static> {
    obj: NonNull<GCHeapedObject<T>>,
}

#[allow(dead_code)]
impl<T> GCArcWeak<T>
where
    T: GCTraceable + 'static,
{
    pub unsafe fn from_raw(obj: NonNull<GCHeapedObject<T>>) -> Self {
        Self { obj }
    }
    pub fn upgrade(&self) -> Option<GCArc<T>> {
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

    pub fn is_valid(&self) -> bool {
        unsafe { self.obj.as_ref().strong_ref() > 0 }
    }
}

impl<T> Clone for GCArcWeak<T>
where
    T: GCTraceable + 'static,
{
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

impl<T> GCRef for GCArcWeak<T>
where
    T: GCTraceable + 'static,
{
    fn strong_ref(&self) -> usize {
        unsafe { self.obj.as_ref().strong_ref() }
    }

    fn weak_ref(&self) -> usize {
        unsafe { self.obj.as_ref().weak_ref() }
    }

    fn inc_ref(&self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .strong_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to increment a GCArcWeak with 0 strong references");
            }
            self.obj
                .as_ref()
                .strong_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn dec_ref(&self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .strong_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to decrement a GCArcWeak with 0 strong references");
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

    fn inc_weak_ref(&self) {
        unsafe {
            self.obj
                .as_ref()
                .weak_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn dec_weak_ref(&self) {
        unsafe {
            if self
                .obj
                .as_ref()
                .weak_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to decrement a GCArcWeak with 0 weak references");
            }
            if self
                .obj
                .as_ref()
                .weak_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
            {
                drop(Box::from_raw(self.obj.as_ptr()));
            }
        }
    }
}

impl<T> Drop for GCArcWeak<T>
where
    T: GCTraceable + 'static,
{
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

unsafe impl<T> Send for GCArcWeak<T> where T: GCTraceable + 'static {}
unsafe impl<T> Sync for GCArcWeak<T> where T: GCTraceable + 'static {}
