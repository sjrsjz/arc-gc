# Rust Arc GC (rust-arc-gc)

[![Crates.io](https://img.shields.io/crates/v/rust-arc-gc.svg)](https://crates.io/crates/rust-arc-gc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Introduction

`rust-arc-gc` is a simple garbage collection (GC) implementation library designed for Rust, providing reference counting functionality similar to Rust's standard library `Arc`, but with added garbage collection capabilities. This library is particularly suitable for handling circular reference problems and applications requiring efficient memory management.

This library combines Rust's memory safety features with the convenience of garbage collection, offering a safe way to manage complex object graphs in Rust.

## Core Features

- **Garbage Collection**: Can detect and release objects that are no longer referenced, including circularly referenced objects
- **Type Safety**: Completely type-safe, no need for `unsafe` code (except within the library itself)
- **Concurrency Safety**: All operations are thread-safe
- **Weak Reference Support**: Provides `GCArcWeak` type for solving circular reference problems
- **Reference Tracking**: Implement the `GCTraceable` trait to make objects part of garbage collection

## Usage

### Installation

Use `cargo add rust-arc-gc` to add the library to your project.

### Basic Example

```rust
use arc_gc::arc::{GCArc, GCArcWeak};
use arc_gc::gc::GC;
use arc_gc::traceable::GCTraceable;

#[allow(dead_code)]
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
            match v.upgrade() {
                Some(ref c) => c.mark_and_visit(),
                None => {
                    panic!("Weak reference is None");
                }
                
            }
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
```

### Handling Circular References

```rust
// Create nodes with circular references
let mut node1 = GCArc::new(Node {
    id: 1,
    children: Vec::new(),
});

let mut node2 = GCArc::new(Node {
    id: 2,
    children: Vec::new(),
});

// Create circular references
node1.downcast_mut::<Node>().children.push(Some(node2.clone()));
node2.downcast_mut::<Node>().children.push(Some(node1.clone()));

// Add to GC
gc.attach(node1.clone());
gc.attach(node2.clone());

// When nodes are no longer referenced externally, GC will reclaim them
drop(node1);
drop(node2);
gc.collect();
```

## API Reference

### GC

- `GC::new()` - Create a new garbage collector instance
- `gc.attach(obj)` - Add an object to the garbage collector's tracking scope
- `gc.collect()` - Perform garbage collection
- `gc.object_count()` - Return the current number of objects managed by the garbage collector
- `gc.create<T>(obj)` - Create a new object and add it to the garbage collector

### GCArc

- `GCArc::new(obj)` - Create a new reference-counted object
- `arc.downcast::<T>()` - Get a reference to the object, with type checking
- `arc.downcast_mut::<T>()` - Get a mutable reference to the object, with type checking
- `arc.mark_and_visit()` - Mark the object and visit its referenced objects
- `arc.is_marked()` - Check if the object is marked
- `arc.as_weak()` - Create a weak reference to the object

### GCTraceable

A trait that must be implemented to allow the garbage collector to track objects:

```rust
pub trait GCTraceable {
    fn visit(&self) {
        // Default implementation is empty, useful for leaf nodes
    }
}
```

### GCArcWeak
- `GCArcWeak::upgrade()` - Upgrade a weak reference to a strong reference, returning `None` if the object has been collected
- `GCArcWeak::is_valid()` - Check if the weak reference is valid (i.e., the object has not been collected)

## Limitations and Future Plans

- The current implementation uses a mark-sweep garbage collection algorithm; generational collection may be added in the future
- Performance optimization: reduce pause time during garbage collection
- Add richer debugging tools and memory usage statistics

## License

This project is licensed under the MIT License.