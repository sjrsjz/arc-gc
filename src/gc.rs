use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Mutex},
};

use rustc_hash::FxHashMap;

use crate::{
    arc::{GCArc, GCRef},
    traceable::GCTraceable,
};

pub struct GC<T: GCTraceable<T> + 'static> {
    gc_refs: Mutex<Vec<GCArc<T>>>,
    attach_count: AtomicUsize,
    collection_percentage: usize, // 百分比阈值，如20表示20%
    memory_threshold: Option<usize>, // 内存阈值（字节），达到此值时触发回收
    allocated_memory: AtomicUsize, // 当前分配的内存大小估算
}

#[allow(dead_code)]
impl<T> GC<T>
where
    T: GCTraceable<T> + 'static,
{    /// 创建一个新的垃圾回收器，默认回收触发百分比为20%
    pub fn new() -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
            attach_count: AtomicUsize::new(0),
            collection_percentage: 20, // 默认20%增长时触发回收
            memory_threshold: None, // 默认不使用内存阈值
            allocated_memory: AtomicUsize::new(0),
        }
    }    /// 创建一个新的垃圾回收器，指定回收触发的百分比
    /// 例如，`new_with_percentage(30)`表示当attach次数超过当前对象数的30%时触发回收
    pub fn new_with_percentage(percentage: usize) -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
            attach_count: AtomicUsize::new(0),
            collection_percentage: percentage,
            memory_threshold: None, // 默认不使用内存阈值
            allocated_memory: AtomicUsize::new(0),
        }
    }

    /// 创建一个新的垃圾回收器，指定内存阈值（字节）
    /// 当分配的内存超过指定阈值时触发回收
    pub fn new_with_memory_threshold(memory_threshold: usize) -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
            attach_count: AtomicUsize::new(0),
            collection_percentage: 20, // 保持默认百分比作为备用触发条件
            memory_threshold: Some(memory_threshold),
            allocated_memory: AtomicUsize::new(0),
        }
    }

    /// 创建一个新的垃圾回收器，同时指定百分比阈值和内存阈值
    /// 任一条件满足时都会触发回收
    pub fn new_with_thresholds(percentage: usize, memory_threshold: usize) -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
            attach_count: AtomicUsize::new(0),
            collection_percentage: percentage,
            memory_threshold: Some(memory_threshold),
            allocated_memory: AtomicUsize::new(0),
        }
    }    pub fn attach(&mut self, gc_arc: &GCArc<T>) {
        {
            let mut gc_refs = self.gc_refs.lock().unwrap();
            gc_refs.push(gc_arc.clone());
        }

        self.attach_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        gc_arc
            .inner()
            .attached_gc_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 更新内存估算（使用对象的大小估算）
        let obj_size = std::mem::size_of::<T>() + std::mem::size_of::<GCArc<T>>();
        self.allocated_memory
            .fetch_add(obj_size, std::sync::atomic::Ordering::Relaxed);

        // 启发式回收检查
        if self.should_collect() {
            self.collect();
        }
    }    pub fn detach(&mut self, gc_arc: &GCArc<T>) -> bool {
        let mut gc_refs = self.gc_refs.lock().unwrap();
        if let Some(index) = gc_refs.iter().position(|r| GCArc::ptr_eq(r, gc_arc)) {
            gc_refs.swap_remove(index);
            gc_arc
                .inner()
                .attached_gc_count
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            
            // 更新内存估算
            let obj_size = std::mem::size_of::<T>() + std::mem::size_of::<GCArc<T>>();
            self.allocated_memory
                .fetch_sub(obj_size, std::sync::atomic::Ordering::Relaxed);
            
            true
        } else {
            false
        }
    }
    pub fn collect(&mut self) {
        // 执行垃圾回收过程。
        // 该过程分为两个主要阶段：标记（Mark）和清除（Sweep）。
        // 1. 标记阶段：从根对象开始，遍历所有可达的对象，并将其标记为“存活”。
        // 2. 清除阶段：遍历所有GC管理的对象，回收所有未被标记为“存活”的对象。

        // 获取对GC管理的引用列表的可变借用。
        // `refs` 存储了所有由GC跟踪的 GCArc<T> 对象。
        let mut refs = self.gc_refs.lock().unwrap();

        // 初始化一个哈希表 `marked` 用于存储每个对象的标记状态。
        // 键是对象的内存地址（usize类型），值是布尔类型（true表示已标记，false表示未标记）。
        // 使用 FxHashMap 是为了更快的哈希性能。
        let mut marked = FxHashMap::default();

        // 初始化标记阶段：将所有GC跟踪的对象在 `marked` 表中初始标记为 `false`（未存活）。
        // 这一步确保了在开始遍历之前，所有对象都被认为是不可达的。
        for r in refs.iter() {
            // 将对象的裸指针（内存地址）作为键。
            marked.insert(r.as_ref() as *const T as usize, false);
        }

        // 初始化一个双端队列 `queue`，用于广度优先搜索（BFS）遍历对象图。
        // 队列中存储的是对象的弱引用 (GCArcWeak<T>)，以避免在遍历过程中增加强引用计数，
        // 从而干扰对象的实际存活状态判断。
        let mut queue = VecDeque::new();

        // 识别根对象（Root Objects）。
        // 根对象是那些除了GC自身持有的引用外，仍然有外部强引用的对象。
        // 在这个实现中，如果一个 GCArc<T> 的强引用计数大于attached_gc_count，
        // （其中attached_gc_count个引用来自各gc的 `gc_refs` 向量，其余来自外部代码），
        // 则认为它是根对象。
        // 将所有根对象的弱引用添加到处理队列 `queue` 中。
        for r in refs.iter() {
            if r.strong_ref()
                > r.inner()
                    .attached_gc_count
                    .load(std::sync::atomic::Ordering::Relaxed)
            {
                // 当强引用计数大于 `attached_gc_count` 时，说明 GC 堆外存在对象（比如VM栈或其他 GCArc 的引用）则认为其为根对象
                queue.push_back(r.as_weak());
            }
        }

        // 开始标记阶段的遍历。
        // 当队列不为空时，持续处理队列中的对象。
        while !queue.is_empty() {
            // 从队列前端取出一个弱引用。
            // `unwrap()` 在这里是安全的，因为我们刚检查了 `!queue.is_empty()`。
            let current_weak = queue.pop_front().unwrap();

            // 尝试将弱引用升级为强引用。
            // 如果升级失败（返回 `None`），意味着该对象已经被释放，
            // 或者在加入队列后、处理前其强引用计数变为0，所以跳过它。
            let Some(current_strong) = current_weak.upgrade() else {
                continue; // 对象已被释放或不再可达
            };

            // 获取当前强引用指向对象的内存地址。
            let current_ptr = current_strong.as_ref() as *const T as usize;

            // 检查该对象是否已经被标记过。
            // `unwrap_or(&false)` 处理了理论上不应发生的情况（对象不在 `marked` 中），
            // 或者对象已在 `marked` 中且值为`true`。
            // 如果对象已经被标记（即 `marked.get(&current_ptr)` 返回 `Some(&true)`），
            // 则跳过，以避免重复处理和循环引用导致的无限循环。
            if *marked.get(&current_ptr).unwrap_or(&false) {
                continue; // 对象已经被访问和标记过了
            }

            // 将当前对象标记为“存活”（设置为 `true`）。
            marked.insert(current_ptr, true);

            // 访问当前对象，并收集它引用的其他GC管理的对象。
            // `GCTraceable::collect` 方法负责将当前对象内部引用的其他
            // `GCArcWeak<T>` 添加到 `queue` 中，以便后续处理。
            current_strong.as_ref().collect(&mut queue);
        }        // 清除阶段（Sweep Phase）。
        // 根据 `marked` 表中的标记状态，筛选出所有存活的对象。
        // `retained` 向量将只包含那些在标记阶段被标记为 `true` 的对象。
        let retained: Vec<GCArc<T>> = refs
            .iter()
            .filter(|r| {
                let ptr = r.as_ref() as *const T as usize;
                // 如果对象在 `marked` 表中为 `true`，则保留它。
                // `unwrap_or(&false)` 确保如果对象由于某种原因不在 `marked` 中（不应发生），
                // 它将被视为未标记，从而被回收。
                let retain = *marked.get(&ptr).unwrap_or(&false);
                if !retain {
                    // 如果对象未被标记为存活，则减少持有的 GC 实例数，因为其将被立即移出堆
                    r.inner()
                        .attached_gc_count
                        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    
                    // 从内存计数中减去被回收对象的大小
                    let obj_size = std::mem::size_of::<T>() + std::mem::size_of::<GCArc<T>>();
                    self.allocated_memory
                        .fetch_sub(obj_size, std::sync::atomic::Ordering::Relaxed);
                }
                retain
            })
            .cloned() // 克隆 GCArc<T> 以便在新向量中拥有它们的所有权。
            .collect();

        // 清空旧的 `refs` 列表。
        refs.clear();
        // 将所有存活的对象添加回 `refs` 列表。
        // 此时，`refs` 只包含标记阶段确认存活的对象。
        // 那些未被标记的对象（即 `retained` 中没有的对象）的 `GCArc` 将会在这里被丢弃。
        // 如果这些是最后的强引用，对象本身将被 `Drop`。
        refs.extend(retained);

        // 重置 `attach_count` 计数器。
        // `attach_count` 用于启发式地决定何时运行垃圾回收。
        // 在一次完整的回收之后，这个计数器被重置为0。
        self.attach_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn object_count(&self) -> usize {
        return self.gc_refs.lock().unwrap().len();
    }

    pub fn get_all(&self) -> Vec<GCArc<T>> {
        self.gc_refs.lock().unwrap().clone()
    }

    pub fn create(&mut self, obj: T) -> GCArc<T> {
        let gc_arc = GCArc::new(obj);
        self.attach(&gc_arc);
        gc_arc
    }

    /// 获取当前分配的内存估算值（字节）
    pub fn allocated_memory(&self) -> usize {
        self.allocated_memory.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 设置内存阈值，None表示禁用内存阈值触发
    pub fn set_memory_threshold(&mut self, threshold: Option<usize>) {
        self.memory_threshold = threshold;
    }

    /// 获取当前内存阈值
    pub fn memory_threshold(&self) -> Option<usize> {
        self.memory_threshold
    }    fn should_collect(&self) -> bool {
        let current_count = self.gc_refs.lock().unwrap().len();
        let attach_count = self.attach_count.load(std::sync::atomic::Ordering::Relaxed);
        let current_memory = self.allocated_memory.load(std::sync::atomic::Ordering::Relaxed);

        if current_count == 0 {
            return false;
        }

        // 检查内存阈值
        if let Some(memory_threshold) = self.memory_threshold {
            if current_memory >= memory_threshold {
                return true;
            }
        }

        // 检查百分比阈值：当attach次数超过当前对象数的指定百分比时触发回收
        let threshold = (current_count * self.collection_percentage) / 100;
        attach_count >= threshold.max(1) // 至少1次attach才触发
    }
}

impl<T> Drop for GC<T>
where
    T: GCTraceable<T> + 'static,
{    fn drop(&mut self) {
        // 在垃圾回收器被销毁时，清理所有跟踪的对象。
        // 这将触发所有对象的 `Drop` 实现。
        let mut refs = self.gc_refs.lock().unwrap();
        for gc_arc in refs.drain(..) {
            // 减少 `attached_gc_count`，表示该对象不再被垃圾回收器跟踪。
            gc_arc
                .inner()
                .attached_gc_count
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            
            // 从内存计数中减去对象大小
            let obj_size = std::mem::size_of::<T>() + std::mem::size_of::<GCArc<T>>();
            self.allocated_memory
                .fetch_sub(obj_size, std::sync::atomic::Ordering::Relaxed);
                
            // 直接调用 `drop` 方法，确保所有对象都被正确释放。
            // 这将触发每个对象的 `Drop` 实现。
            drop(gc_arc);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::{arc::GCArcWeak, traceable::GCTraceable};

    struct TestObject {
        value: Option<GCArcWeak<TestObjectCell>>,
    }

    impl GCTraceable<TestObjectCell> for TestObject {
        fn collect(&self, queue: &mut VecDeque<GCArcWeak<TestObjectCell>>) {
            if let Some(ref weak_ref) = self.value {
                queue.push_back(weak_ref.clone());
            }
        }
    }

    impl Drop for TestObject {
        fn drop(&mut self) {
            println!("Dropping TestObject: address={:p}", self);
        }
    }

    struct TestObjectCell(RefCell<TestObject>);
    impl GCTraceable<TestObjectCell> for TestObjectCell {
        fn collect(&self, queue: &mut VecDeque<GCArcWeak<TestObjectCell>>) {
            if let Ok(obj) = self.0.try_borrow() {
                if let Some(ref weak_ref) = obj.value {
                    queue.push_back(weak_ref.clone());
                }
            }
        }
    }
    impl Drop for TestObjectCell {
        fn drop(&mut self) {
            println!("Dropping TestObjectCell: address={:p}", self);
        }
    }

    #[test]
    fn test_gc() {
        let mut gc: GC<TestObjectCell> = GC::new_with_percentage(20);
        {
            let obj1 = gc.create(TestObjectCell {
                0: RefCell::new(TestObject { value: None }),
            });
            let weak_ref = obj1.as_weak();
            match obj1.as_ref().0.try_borrow_mut() {
                Ok(mut obj) => {
                    obj.value = Some(weak_ref);
                }
                Err(_) => {
                    panic!("Failed to borrow TestObjectCell mutably");
                }
            }
            print!("GC object count before collection: {}\n", gc.object_count());
        }
        gc.collect();
        println!("GC completed, all objects should be dropped now.");
    }

    #[test]
    fn test_memory_threshold_gc() {
        // 使用较小的内存阈值（1KB）来测试内存触发
        let mut gc: GC<TestObjectCell> = GC::new_with_memory_threshold(1024);
        
        println!("Initial allocated memory: {} bytes", gc.allocated_memory());
        
        // 创建多个对象直到触发内存阈值
        let mut objects = Vec::new();
        for i in 0..50 {
            let obj = gc.create(TestObjectCell {
                0: RefCell::new(TestObject { value: None }),
            });
            objects.push(obj);
            
            println!("After creating object {}: allocated={} bytes, object_count={}", 
                    i + 1, gc.allocated_memory(), gc.object_count());
            
            if gc.allocated_memory() > 1024 {
                break;
            }
        }
        
        println!("Before collection: allocated={} bytes, object_count={}", 
                gc.allocated_memory(), gc.object_count());
        
        // 释放引用，让对象变成垃圾
        objects.clear();
        
        // 手动触发回收
        gc.collect();
        
        println!("After collection: allocated={} bytes, object_count={}", 
                gc.allocated_memory(), gc.object_count());
    }

    #[test]
    fn test_combined_thresholds_gc() {
        // 测试同时使用百分比和内存阈值
        let mut gc: GC<TestObjectCell> = GC::new_with_thresholds(50, 2048); // 50%或2KB
        
        println!("Testing combined thresholds: 50% or 2KB");
        
        let obj1 = gc.create(TestObjectCell {
            0: RefCell::new(TestObject { value: None }),
        });
        
        println!("Memory threshold: {:?}", gc.memory_threshold());
        println!("Allocated memory: {} bytes", gc.allocated_memory());
        println!("Object count: {}", gc.object_count());
        
        // 保持引用以防止被回收
        let _keep_ref = obj1;
    }
}
