# Arc GC (arc-gc)

[![Crates.io](https://img.shields.io/crates/v/arc-gc.svg)](https://crates.io/crates/arc-gc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Introduction

`arc-gc` is a simple garbage collection (GC) implementation library designed for Rust, providing reference counting functionality similar to Rust's standard library `Arc`, but with added garbage collection capabilities. This library is particularly suitable for handling circular reference problems and applications requiring efficient memory management.

This library combines Rust's memory safety features with the convenience of garbage collection, offering a safe way to manage complex object graphs in Rust.

## Core Features

- **Garbage Collection**: Can detect and release objects that are no longer referenced, including circularly referenced objects
- **Type Safety**: Completely type-safe, no need for `unsafe` code (except within the library itself)
- **Concurrency Safety**: All operations are thread-safe
- **Weak Reference Support**: Provides `GCArcWeak` type for solving circular reference problems
- **Reference Tracking**: Implement the `GCTraceable` trait to make objects part of garbage collection

## Usage

### Installation

Add the following dependency to your Cargo.toml file:

```toml
[dependencies]
arc-gc = "0.1.0"
```

### Basic Example

```rust
use arc_gc::gc::GC;
use arc_gc::arc::GCArc;
use arc_gc::traceable::GCTraceable;

// Define a traceable object
struct Node {
    id: usize,
    children: Vec<Option<GCArc>>,
}

// Implement GCTraceable trait to allow the garbage collector to track references
impl GCTraceable for Node {
    fn visit(&self) {
        // Traverse all referenced objects and mark them
        for child in &self.children {
            if let Some(ref child_ref) = child {
                child_ref.mark_and_visit();
            }
        }
    }
}

// Using the garbage collector
fn main() {
    // Create a garbage collector
    let mut gc = GC::new();
    
    // Create a root node
    let root = GCArc::new(Node {
        id: 1,
        children: Vec::new(),
    });
    
    // Create a child node
    let child = GCArc::new(Node {
        id: 2,
        children: Vec::new(),
    });
    
    // Add the child node to the root node
    root.downcast_mut::<Node>().children.push(Some(child.clone()));
    
    // Add objects to the garbage collector
    gc.attach(root.clone());
    gc.attach(child.clone());
    
    // Run garbage collection
    gc.collect();
    
    // View object count
    println!("Object count: {}", gc.object_count());
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

## Limitations and Future Plans

- The current implementation uses a mark-sweep garbage collection algorithm; generational collection may be added in the future
- Performance optimization: reduce pause time during garbage collection
- Add richer debugging tools and memory usage statistics

## License

This project is licensed under the MIT License.