use std::{
    ptr::NonNull,
    sync::atomic::Ordering,
};

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
            if self.obj.as_ref().dropped.load(Ordering::Acquire) {
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
                if self.obj.as_ref().dropped.load(Ordering::Acquire) {
                    // 如果对象已被释放，撤销计数增加
                    self.obj.as_ref().strong_rc.fetch_sub(1, Ordering::SeqCst);
                    return None;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traceable::GCTraceable;

    #[derive(Debug, PartialEq)]
    struct TestValue {
        value: i32,
    }

    impl GCTraceable for TestValue {
        fn visit(&self) {
            // Empty implementation for testing
        }
    }

    /// 测试原始的未定义行为场景 - 多个 GCArc 指向同一对象时不应允许可变引用
    #[test]
    fn test_no_mutable_reference_with_multiple_strong_refs() {
        let mut a = GCArc::new(TestValue { value: 1 });
        let _a_clone = a.clone(); // 创建第二个强引用

        // 现在 strong_count > 1，所以 try_as_mut 应该返回 None
        assert!(a.try_as_mut().is_none());
    }

    /// 测试当存在弱引用时不应允许可变引用
    #[test]
    fn test_no_mutable_reference_with_weak_refs() {
        let mut a = GCArc::new(TestValue { value: 1 });
        let _weak = a.as_weak(); // 创建弱引用

        // 现在有弱引用存在，所以 try_as_mut 应该返回 None
        assert!(a.try_as_mut().is_none());
    }

    /// 测试唯一引用时应该允许可变引用
    #[test]
    fn test_mutable_reference_when_unique() {
        let mut a = GCArc::new(TestValue { value: 1 });

        // 只有一个强引用且没有弱引用，应该可以获取可变引用
        let mutable_ref = a.try_as_mut();
        assert!(mutable_ref.is_some());

        if let Some(val) = mutable_ref {
            val.value = 42;
            assert_eq!(val.value, 42);
        }
    }

    /// 测试 get_mut 在不唯一时会 panic
    #[test]
    #[should_panic(expected = "Cannot get mutable reference: GCArc is not unique")]
    fn test_get_mut_panics_when_not_unique() {
        let mut a = GCArc::new(TestValue { value: 1 });
        let _a_clone = a.clone();

        // 这应该会 panic
        let _mutable_ref = a.get_mut();
    }

    /// 测试在强引用被释放后可以重新获取可变引用
    #[test]
    fn test_mutable_reference_after_clone_dropped() {
        let mut a = GCArc::new(TestValue { value: 1 });

        {
            let _a_clone = a.clone();
            // 在这个作用域内，try_as_mut 应该返回 None
            assert!(a.try_as_mut().is_none());
        } // a_clone 在这里被释放

        // 现在应该可以获取可变引用了
        let mutable_ref = a.try_as_mut();
        assert!(mutable_ref.is_some());
    }

    /// 测试在弱引用被释放后可以重新获取可变引用
    #[test]
    fn test_mutable_reference_after_weak_dropped() {
        let mut a = GCArc::new(TestValue { value: 1 });

        {
            let _weak = a.as_weak();
            // 在这个作用域内，try_as_mut 应该返回 None
            assert!(a.try_as_mut().is_none());
        } // weak 在这里被释放

        // 现在应该可以获取可变引用了
        let mutable_ref = a.try_as_mut();
        assert!(mutable_ref.is_some());
    }

    /// 模拟原始的 UB 场景，但现在应该是安全的
    #[test]
    fn test_original_ub_scenario_now_safe() {
        let mut a = GCArc::new(TestValue { value: 1 });
        let a_clone = a.clone();

        // 尝试获取可变引用 - 应该失败因为有多个强引用
        let mutable_ref = a.try_as_mut();
        assert!(
            mutable_ref.is_none(),
            "Should not be able to get mutable reference when multiple strong refs exist"
        );

        // 获取不可变引用 - 这应该总是可以的
        let immutable_ref = a_clone.as_ref();
        assert_eq!(immutable_ref.value, 1);

        // 原始代码中的 UB 现在被防止了
        // 如果 try_as_mut 返回了 Some，那么就会有 UB，但现在它返回 None
    }

    /// 测试引用计数的正确性
    #[test]
    fn test_reference_counts() {
        let a = GCArc::new(TestValue { value: 1 });

        // 初始状态：1个强引用，0个弱引用
        assert_eq!(a.strong_ref(), 1);
        assert_eq!(a.weak_ref(), 0);

        let a_clone = a.clone();
        // 克隆后：2个强引用，0个弱引用
        assert_eq!(a.strong_ref(), 2);
        assert_eq!(a.weak_ref(), 0);

        let weak = a.as_weak();
        // 创建弱引用后：2个强引用，1个弱引用
        assert_eq!(a.strong_ref(), 2);
        assert_eq!(a.weak_ref(), 1);

        drop(a_clone);
        // 释放一个强引用后：1个强引用，1个弱引用
        assert_eq!(a.strong_ref(), 1);
        assert_eq!(a.weak_ref(), 1);

        drop(weak);
        // 释放弱引用后：1个强引用，0个弱引用
        assert_eq!(a.strong_ref(), 1);
        assert_eq!(a.weak_ref(), 0);
    }

    /// 测试复杂场景：多个强引用和弱引用的组合
    #[test]
    fn test_complex_reference_scenario() {
        let mut a = GCArc::new(TestValue { value: 1 });

        // 场景1：多个强引用
        let a2 = a.clone();
        let a3 = a.clone();
        assert!(a.try_as_mut().is_none());

        drop(a2);
        assert!(a.try_as_mut().is_none()); // 仍然有 a3

        drop(a3);
        assert!(a.try_as_mut().is_some()); // 现在只有 a

        // 场景2：弱引用的影响
        let weak1 = a.as_weak();
        let weak2 = a.as_weak();
        assert!(a.try_as_mut().is_none()); // 有弱引用存在

        drop(weak1);
        assert!(a.try_as_mut().is_none()); // 仍然有 weak2

        drop(weak2);
        assert!(a.try_as_mut().is_some()); // 现在没有其他引用
    }

    /// 演示原始 UB 问题（如果没有修复的话）
    #[test]
    fn test_demonstrate_original_ub_prevention() {
        let mut a = GCArc::new(TestValue { value: 1 });
        let mut b = a.clone();

        // 在修复之前，以下代码会导致 UB：
        // let ref_a = a.as_mut(); // 第一个可变引用
        // let ref_b = b.as_mut(); // 第二个可变引用指向同一对象！
        // 这会违反 Rust 的借用规则

        // 现在有了修复，这种情况被防止了：
        assert!(a.try_as_mut().is_none()); // 因为有多个强引用
        assert!(b.try_as_mut().is_none()); // 因为有多个强引用

        // 只有当只剩一个引用时才能获取可变引用
        drop(b);
        assert!(a.try_as_mut().is_some()); // 现在可以了
    }
    /// 测试弱引用升级的竞态条件修复
    #[test]
    fn test_weak_upgrade_race_condition_fix() {
        let a = GCArc::new(TestValue { value: 1 });
        let weak = a.as_weak();

        // 升级应该成功
        let upgraded = weak.upgrade();
        assert!(upgraded.is_some());

        drop(a);
        drop(upgraded); // 还需要释放升级的引用

        // 现在升级应该失败
        let upgraded = weak.upgrade();
        assert!(upgraded.is_none());
    }
}
