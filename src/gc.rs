use std::sync::{atomic::AtomicUsize, Mutex};

use crate::{
    arc::{GCArc, GCRef},
    traceable::GCTraceable,
};

pub struct GC<T: GCTraceable + 'static> {
    gc_refs: Mutex<Vec<GCArc<T>>>,
    attach_count: AtomicUsize,
    collection_percentage: usize, // 百分比阈值，如20表示20%
}

#[allow(dead_code)]
impl<T> GC<T>
where
    T: GCTraceable + 'static,
{
    /// 创建一个新的垃圾回收器，默认回收触发百分比为20%
    pub fn new() -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
            attach_count: AtomicUsize::new(0),
            collection_percentage: 20, // 默认20%增长时触发回收
        }
    }

    /// 创建一个新的垃圾回收器，指定回收触发的百分比
    /// 例如，`new_with_percentage(30)`表示当attach次数超过当前对象数的30%时触发回收
    pub fn new_with_percentage(percentage: usize) -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
            attach_count: AtomicUsize::new(0),
            collection_percentage: percentage,
        }
    }

    pub fn attach(&mut self, gc_arc: GCArc<T>) {
        {
            let mut gc_refs = self.gc_refs.lock().unwrap();
            gc_refs.push(gc_arc);
        }

        self.attach_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 启发式回收检查
        if self.should_collect() {
            self.collect();
        }
    }

    pub fn detach(&mut self, gc_arc: &GCArc<T>) -> bool {
        let mut gc_refs = self.gc_refs.lock().unwrap();
        if let Some(index) = gc_refs.iter().position(|r| GCArc::ptr_eq(r, gc_arc)) {
            gc_refs.swap_remove(index);
            true
        } else {
            false
        }
    }
    pub fn collect(&mut self) {
        // Implement garbage collection logic here
        // For example, iterate over weak references and clean up if necessary
        let mut refs = self.gc_refs.lock().unwrap();

        for r in refs.iter_mut() {
            r.unmark();
        }

        let mut roots: Vec<&mut GCArc<T>> = refs
            .iter_mut()
            .filter(
                |r| r.strong_ref() > 1, // Assuming strong_ref() > 1 means it's a root
            )
            .collect();

        for r in roots.iter_mut() {
            r.mark_and_visit();
        }

        let retained: Vec<GCArc<T>> = refs
            .iter_mut()
            .filter(|r| r.is_marked())
            .map(|r| r.clone())
            .collect();

        refs.clear();
        refs.extend(retained);

        // 重置计数器
        self.attach_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn object_count(&self) -> usize {
        return self.gc_refs.lock().unwrap().len();
    }

    pub fn get_all(&self) -> Vec<GCArc<T>> {
        self.gc_refs.lock().unwrap().clone()
    }

    pub fn create(&mut self, obj: T) -> GCArc<T> {
        let gc_arc = GCArc::new(obj);
        self.attach(gc_arc.clone());
        gc_arc
    }

    fn should_collect(&self) -> bool {
        let current_count = self.gc_refs.lock().unwrap().len();
        let attach_count = self.attach_count.load(std::sync::atomic::Ordering::Relaxed);

        if current_count == 0 {
            return false;
        }

        // 当attach次数超过当前对象数的指定百分比时触发回收
        let threshold = (current_count * self.collection_percentage) / 100;
        attach_count >= threshold.max(1) // 至少1次attach才触发
    }
}
