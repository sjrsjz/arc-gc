#[cfg(test)]
mod advanced_tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    
    use crate::gc::GC;
    use crate::gc_ref::{GCArc, GCRef, GCTraceable};

    // 可以记录是否被删除的复杂结构
    struct ComplexNode {
        id: usize,
        children: Vec<GCArcWeak>,
        // 使用Rc<RefCell<bool>>来追踪节点是否被删除
        dropped: Rc<RefCell<bool>>,
    }
    
    impl GCTraceable for ComplexNode {
        fn visit(&self) {
            // 递归访问所有子节点
            for child in &self.children {
                child.mark_and_visit();
            }
            
        }
    }
    
    impl Drop for ComplexNode {
        fn drop(&mut self) {
            println!("Dropping ComplexNode {}", self.id);
            // 标记此节点已被删除
            *self.dropped.borrow_mut() = true;
        }
    }
    
    use crate::gc_ref::GCArcWeak;
    
    #[test]
    fn test_advanced_gc_with_cycles() {
        let mut gc = GC::new();
        
        // 创建追踪变量，用于验证哪些节点被删除
        let dropped1 = Rc::new(RefCell::new(false));
        let dropped2 = Rc::new(RefCell::new(false));
        let dropped3 = Rc::new(RefCell::new(false));
        let dropped4 = Rc::new(RefCell::new(false));
        
        // 创建节点1（根节点）
        let mut node1 = GCArc::new(ComplexNode {
            id: 1,
            children: Vec::new(),
            dropped: dropped1.clone(),
        });
        
        // 创建节点2（根节点的子节点）
        let mut node2 = GCArc::new(ComplexNode {
            id: 2,
            children: Vec::new(),
            dropped: dropped2.clone(),
        });
        
        // 创建节点3（节点2的子节点）
        let mut node3 = GCArc::new(ComplexNode {
            id: 3,
            children: Vec::new(),
            dropped: dropped3.clone(),
        });
        
        // 创建节点4（不可达节点）
        let mut node4 = GCArc::new(ComplexNode {
            id: 4,
            children: Vec::new(),
            dropped: dropped4.clone(),
        });

        // 设置节点之间的引用关系
        node1.downcast_mut::<ComplexNode>().children.push(node2.as_weak());
        node2.downcast_mut::<ComplexNode>().children.push(node3.as_weak());
        node3.downcast_mut::<ComplexNode>().children.push(node1.as_weak());
        node4.downcast_mut::<ComplexNode>().children.push(node1.as_weak());

        // 挂载到GC阻止自动回收
        gc.attach(node1.clone());
        gc.attach(node2.clone());
        gc.attach(node3.clone());
        gc.attach(node4.clone());
        // 人为drop掉节点1和节点2和节点3
        drop(node1);
        drop(node2);
        drop(node3);
        // 此时节点4应该是唯一强引用的节点
        gc.collect();
        // 验证节点4是否被正确收集
        assert_eq!(*dropped1.borrow(), false, "节点1不应该被GC收集");
        assert_eq!(*dropped2.borrow(), false, "节点2不应该被GC收集");
        assert_eq!(*dropped3.borrow(), false, "节点3不应该被GC收集");
        assert_eq!(*dropped4.borrow(), false, "节点4不应该被GC收集");
        // 验证节点4的引用计数
        assert_eq!(node4.strong_ref(), 2, "节点4的引用计数应该是2(一个本作用域和一个被GC引用)");

        // drop掉节点4
        drop(node4);
        gc.collect();
        // 验证节点4是否被正确收集
        assert_eq!(*dropped1.borrow(), true, "节点1应该被GC收集");
        assert_eq!(*dropped2.borrow(), true, "节点2应该被GC收集");
        assert_eq!(*dropped3.borrow(), true, "节点3应该被GC收集");
        assert_eq!(*dropped4.borrow(), true, "节点4应该被GC收集");

        

    }
    
    #[test]
    fn test_gc_memory_leak_detection() {
        let mut gc = GC::new();
        
        // 创建一系列的节点，通过设置弱引用来模拟内存泄漏场景
        
        // 创建追踪变量
        let leaked_dropped = Rc::new(RefCell::new(false));
        
        // 创建一个孤立的节点，没有任何强引用指向它
        {
            let leaked_node = GCArc::new(ComplexNode {
                id: 999,
                children: Vec::new(),
                dropped: leaked_dropped.clone(),
            });
            
            // 只附加一个弱引用到GC
            gc.attach(leaked_node.clone());
            
            // leaked_node在这里离开作用域，它的强引用计数应该变为0
        }
        
        // 收集垃圾
        gc.collect();
        
        // 验证泄漏的节点是否被正确收集
        assert_eq!(*leaked_dropped.borrow(), true, "泄漏的节点应该被GC收集");
    }
}
