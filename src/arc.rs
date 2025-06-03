use std::{collections::VecDeque, ptr::NonNull, sync::atomic::Ordering};

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

pub struct GCArc<T: GCTraceable<T> + 'static> {
    obj: NonNull<GCHeapedObject<T>>,
}

#[allow(dead_code)]
impl<T> GCArc<T>
where
    T: GCTraceable<T> + 'static,
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

    pub fn as_ref(&self) -> &T {
        unsafe { self.obj.as_ref().as_ref() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.try_as_mut().expect(
            "Cannot get mutable reference: GCArc is not unique. \
             Strong count > 1 or weak references exist. \
             Consider using interior mutability (RefCell, Mutex, etc.) instead.",
        )
    }

    pub fn try_as_mut(&mut self) -> Option<&mut T> {
        let strong_count = unsafe { self.obj.as_ref().strong_rc.load(Ordering::SeqCst) };
        let weak_count = unsafe { self.obj.as_ref().weak_rc.load(Ordering::SeqCst) };

        // 只有当强引用计数为1且没有弱引用时才允许可变访问
        if strong_count == 1 && weak_count == 0 {
            Some(unsafe { self.obj.as_mut().as_mut() })
        } else {
            None
        }
    }

    fn collect(&self, queue: &mut VecDeque<GCArcWeak<T>>) {
        unsafe {
            self.obj.as_ref().as_ref().collect(queue);
        }
    }

    pub(crate) fn ptr_eq(a: &GCArc<T>, b: &GCArc<T>) -> bool {
        unsafe { std::ptr::eq(a.obj.as_ref(), b.obj.as_ref()) }
    }
}

impl<T> Clone for GCArc<T>
where
    T: GCTraceable<T> + 'static,
{
    fn clone(&self) -> Self {
        unsafe {
            self.obj
                .as_ref()
                .strong_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.obj
                .as_ref()
                .weak_rc
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        Self { obj: self.obj }
    }
}

impl<T> GCRef for GCArc<T>
where
    T: GCTraceable<T> + 'static,
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
    T: GCTraceable<T> + 'static,
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
            }
            if self
                .obj
                .as_ref()
                .weak_rc
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                panic!("Attempted to drop a GCArc with 0 weak references");
            }
            // 减少弱引用计数
            if self
                .obj
                .as_ref()
                .weak_rc
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
            {
                // 如果弱引用计数降到0，释放对象
                drop(Box::from_raw(self.obj.as_ptr()));
            }
        }
    }
}

unsafe impl<T> Send for GCArc<T> where T: GCTraceable<T> + 'static {}
unsafe impl<T> Sync for GCArc<T> where T: GCTraceable<T> + 'static {}

pub struct GCArcWeak<T: GCTraceable<T> + 'static> {
    obj: NonNull<GCHeapedObject<T>>,
}

#[allow(dead_code)]
impl<T> GCArcWeak<T>
where
    T: GCTraceable<T> + 'static,
{
    pub unsafe fn from_raw(obj: NonNull<GCHeapedObject<T>>) -> Self {
        Self { obj }
    }
    pub fn upgrade(&self) -> Option<GCArc<T>> {
        #[inline]
        fn checked_increment(n: usize) -> Option<usize> {
            // 如果强引用计数为0，对象已被释放
            if n == 0 {
                return None;
            }
            // 防止引用计数溢出
            if n >= usize::MAX / 2 {
                panic!("Reference count overflow");
            }
            Some(n + 1)
        }

        unsafe {
            // 首先检查对象是否已被标记为释放
            if self.obj.as_ref().is_dropped() {
                return None;
            }

            // 使用 fetch_update 原子地尝试增加强引用计数
            // 这比手动循环更高效且更安全
            if self
                .obj
                .as_ref()
                .strong_rc
                .fetch_update(Ordering::SeqCst, Ordering::Relaxed, checked_increment)
                .is_ok()
            {
                // 再次检查对象是否在我们增加计数后被释放
                if self.obj.as_ref().is_dropped() {
                    // 如果对象已被释放，撤销计数增加
                    self.obj.as_ref().strong_rc.fetch_sub(1, Ordering::SeqCst);
                    return None;
                }
                // 弱引用计数也需要增加
                self.obj
                    .as_ref()
                    .weak_rc
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(GCArc { obj: self.obj })
            } else {
                None
            }
        }
    }
    pub fn is_valid(&self) -> bool {
        unsafe { self.obj.as_ref().strong_ref() > 0 }
    }
}

impl<T> Clone for GCArcWeak<T>
where
    T: GCTraceable<T> + 'static,
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
    T: GCTraceable<T> + 'static,
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
    T: GCTraceable<T> + 'static,
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
                drop(Box::from_raw(self.obj.as_ptr()));
            }
        }
    }
}

unsafe impl<T> Send for GCArcWeak<T> where T: GCTraceable<T> + 'static {}
unsafe impl<T> Sync for GCArcWeak<T> where T: GCTraceable<T> + 'static {}
