use crate::gc_ref::{GCArc, GCArcWeak, GCRef, GCTraceable};
use crate::gc::GC;

struct GCInt {
    value: i32
}
impl GCTraceable for GCInt {}

struct GCList {
    values : Vec<GCArcWeak>
}
impl GCTraceable for GCList {
    fn visit(&self) {
        for v in &self.values {
            v.mark_and_visit();
        }
    }
}

impl GCList {
    pub fn append(&mut self, v: GCArcWeak) {
        self.values.push(v);
    }

}

#[test]
fn test_gc_list(){
    // 创建一个垃圾回收器
    let mut gc = GC::new();
    
    // 创建几个 GCInt 对象
    let int1 = GCArc::new(GCInt { value: 1 });
    let int2 = GCArc::new(GCInt { value: 2 });
    let int3 = GCArc::new(GCInt { value: 3 });
    
    // 创建一个空列表
    let mut list = GCArc::new(GCList { values: Vec::new() });
    
    // 向列表添加一些整数对象的弱引用
    {
        let list_ref = list.downcast_mut::<GCList>();
        list_ref.append(int1.as_weak());
        list_ref.append(int2.as_weak());
        list_ref.append(int3.as_weak());
    }
    
    // 将所有对象附加到 GC 中
    gc.attach(int1.clone());
    gc.attach(int2.clone());
    gc.attach(int3.clone());
    gc.attach(list.clone());
    
    // 在此点，所有对象都有强引用，不应被回收
    gc.collect();
    
    // 确认所有对象都存活
    assert_eq!(gc.object_count(), 4);
    
    // 丢弃 int1 的强引用
    std::mem::drop(int1);
    
    // 此时 int1 应该保留，因为 list 仍然持有其弱引用
    gc.collect();
    assert_eq!(gc.object_count(), 4);
    
    // 丢弃 list 的强引用
    std::mem::drop(list);
    gc.collect();
    
    // 现在 list 和 int1 都应该被回收，因为没有可达的强引用
    assert_eq!(gc.object_count(), 2);
    
    // 丢弃所有剩余的强引用
    std::mem::drop(int2);
    std::mem::drop(int3);
    gc.collect();
    
    // 所有对象都应该被回收
    assert_eq!(gc.object_count(), 0);
}