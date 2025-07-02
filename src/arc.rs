use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Arc, Weak},
};

use crate::traceable::GCTraceable;

/// GCWrapper 包装器，包含被垃圾回收的对象和附加的GC计数
pub struct GCWrapper<T: GCTraceable<T> + 'static> {
    value: T,
    pub(crate) attached_gc_count: AtomicUsize,
}

impl<T: GCTraceable<T> + 'static> GCWrapper<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            attached_gc_count: AtomicUsize::new(0),
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

#[allow(dead_code)]
pub trait GCRef {
    fn strong_ref(&self) -> usize;
    fn weak_ref(&self) -> usize;
}

pub struct GCArc<T: GCTraceable<T> + 'static> {
    inner: Arc<GCWrapper<T>>,
}

impl<T: GCTraceable<T> + 'static> Into<GCArc<T>> for Arc<GCWrapper<T>> {
    fn into(self) -> GCArc<T> {
        GCArc { inner: self }
    }
}

impl<T: GCTraceable<T> + 'static> From<GCArc<T>> for Arc<GCWrapper<T>> {
    fn from(gc_arc: GCArc<T>) -> Self {
        gc_arc.inner
    }
}

#[allow(dead_code)]
impl<T> GCArc<T>
where
    T: GCTraceable<T> + 'static,
{
    pub fn new(obj: T) -> Self {
        Self {
            inner: Arc::new(GCWrapper::new(obj)),
        }
    }
    pub fn as_weak(&self) -> GCArcWeak<T> {
        GCArcWeak {
            inner: Arc::downgrade(&self.inner),
        }
    }

    pub fn as_ref(&self) -> &T {
        &self.inner.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.try_as_mut().expect(
            "Cannot get mutable reference: GCArc is not unique. \
             Strong count > 1 or weak references exist. \
             Consider using interior mutability (RefCell, Mutex, etc.) instead.",
        )
    }

    pub fn try_as_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.inner).map(|wrapper| &mut wrapper.value)
    }

    fn collect(&self, queue: &mut VecDeque<GCArcWeak<T>>) {
        self.inner.value.collect(queue);
    }

    pub(crate) fn ptr_eq(a: &GCArc<T>, b: &GCArc<T>) -> bool {
        Arc::ptr_eq(&a.inner, &b.inner)
    }

    #[inline(always)]
    pub(crate) fn inner(&self) -> &GCWrapper<T> {
        &self.inner
    }
}

impl<T> Clone for GCArc<T>
where
    T: GCTraceable<T> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> GCRef for GCArc<T>
where
    T: GCTraceable<T> + 'static,
{
    fn strong_ref(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    fn weak_ref(&self) -> usize {
        Arc::weak_count(&self.inner)
    }
}

pub struct GCArcWeak<T: GCTraceable<T> + 'static> {
    inner: Weak<GCWrapper<T>>,
}

impl<T: GCTraceable<T> + 'static> Into<GCArcWeak<T>> for Weak<GCWrapper<T>> {
    fn into(self) -> GCArcWeak<T> {
        GCArcWeak { inner: self }
    }
}

impl<T: GCTraceable<T> + 'static> From<GCArcWeak<T>> for Weak<GCWrapper<T>> {
    fn from(gc_arc_weak: GCArcWeak<T>) -> Self {
        gc_arc_weak.inner
    }
}

#[allow(dead_code)]
impl<T> GCArcWeak<T>
where
    T: GCTraceable<T> + 'static,
{
    pub fn upgrade(&self) -> Option<GCArc<T>> {
        self.inner.upgrade().map(|inner| GCArc { inner })
    }

    pub fn is_valid(&self) -> bool {
        self.inner.strong_count() > 0
    }
}

impl<T> Clone for GCArcWeak<T>
where
    T: GCTraceable<T> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> GCRef for GCArcWeak<T>
where
    T: GCTraceable<T> + 'static,
{
    fn strong_ref(&self) -> usize {
        self.inner.strong_count()
    }

    fn weak_ref(&self) -> usize {
        self.inner.weak_count()
    }
}
