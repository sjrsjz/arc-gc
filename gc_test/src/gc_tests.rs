#[cfg(test)]
mod tests {
    use arc_gc::gc::GC;
    use arc_gc::arc::GCArc;
    use arc_gc::traceable::GCTraceable;

    // 测试用的简单结构，可以形成图形结构
    struct TestNode {
        id: usize,
        references: Vec<Option<GCArc>>,
    }

    impl Drop for TestNode {
        fn drop(&mut self) {
            println!("Dropping TestNode {}", self.id);
        }
    }

    impl GCTraceable for TestNode {
        fn visit(&self) {
            // 遍历引用，标记所有相关节点
            for ref_opt in &self.references {
                if let Some(ref r) = ref_opt {
                    r.mark_and_visit();
                }
            }
        }
    }

    #[test]
    fn test_gc_simple_collection() {
        let mut gc = GC::new();
        
        // 创建根节点
        let mut root = GCArc::new(TestNode {
            id: 1,
            references: Vec::new(),
        });
        
        // 创建几个子节点
        let node2 = GCArc::new(TestNode {
            id: 2,
            references: Vec::new(),
        });
        
        let node3 = GCArc::new(TestNode {
            id: 3,
            references: Vec::new(),
        });
        
        // 将子节点添加到根节点
        root.downcast_mut::<TestNode>().references.push(Some(node2.clone()));
        root.downcast_mut::<TestNode>().references.push(Some(node3.clone()));
        
        // 将根节点添加到GC
        gc.attach(root.clone());
        
        // 确保收集前可以访问
        assert_eq!(root.downcast::<TestNode>().id, 1);
        assert_eq!(node2.downcast::<TestNode>().id, 2);
        assert_eq!(node3.downcast::<TestNode>().id, 3);
        
        // 收集垃圾，但因为有根节点引用，所以不应该收集任何东西
        gc.collect();
        
        // 确保收集后仍然存在
        assert_eq!(root.downcast::<TestNode>().id, 1);
        assert_eq!(node2.downcast::<TestNode>().id, 2);
        assert_eq!(node3.downcast::<TestNode>().id, 3);
    }
    
    #[test]
    fn test_gc_unreachable_collection() {
        let mut gc = GC::new();
        
        // 创建一些节点
        let mut root = GCArc::new(TestNode {
            id: 1,
            references: Vec::new(),
        });
        
        let node2 = GCArc::new(TestNode {
            id: 2,
            references: Vec::new(),
        });
        
        let node3 = GCArc::new(TestNode {
            id: 3,
            references: Vec::new(),
        });
        
        // 创建一个不可达节点
        let unreachable_node = GCArc::new(TestNode {
            id: 4,
            references: Vec::new(),
        });
        
        // 将node2和node3添加到根节点
        root.downcast_mut::<TestNode>().references.push(Some(node2.clone()));
        root.downcast_mut::<TestNode>().references.push(Some(node3.clone()));
        
        // 将所有节点添加到GC
        gc.attach(root.clone());
        gc.attach(unreachable_node.clone());
        
        // 收集垃圾，unreachable_node应该被收集
        gc.collect();
        
        // 验证根节点和可达节点仍然存在
        assert_eq!(root.downcast::<TestNode>().id, 1);
        assert_eq!(node2.downcast::<TestNode>().id, 2);
        assert_eq!(node3.downcast::<TestNode>().id, 3);
    }
    
    #[test]
    fn test_gc_cyclic_references() {
        let mut gc = GC::new();
        
        // 创建根节点
        let root = GCArc::new(TestNode {
            id: 1,
            references: Vec::new(),
        });
        
        // 创建形成循环的两个节点
        let mut node2 = GCArc::new(TestNode {
            id: 2,
            references: Vec::new(),
        });
        
        let mut node3 = GCArc::new(TestNode {
            id: 3,
            references: Vec::new(),
        });
        
        // 使node2指向node3，node3指向node2，形成循环
        node2.downcast_mut::<TestNode>().references.push(Some(node3.clone()));
        node3.downcast_mut::<TestNode>().references.push(Some(node2.clone()));
        
        // 将根节点和node2添加到GC
        gc.attach(root.clone());
        gc.attach(node2.clone());
        
        // 收集垃圾前，验证节点存在
        assert_eq!(root.downcast::<TestNode>().id, 1);
        assert_eq!(node2.downcast::<TestNode>().id, 2);
        assert_eq!(node3.downcast::<TestNode>().id, 3);
        
        // 当根节点失去引用时，根节点应该被收集
        // 但node2和node3因为循环引用，应该保留
        gc.collect();
        
        // 验证node2和node3仍然存在
        assert_eq!(node2.downcast::<TestNode>().id, 2);
        assert_eq!(node3.downcast::<TestNode>().id, 3);
    }
}
