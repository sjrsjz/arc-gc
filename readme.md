# Rust Arc GC (rust-arc-gc)

[![Crates.io](https://img.shields.io/crates/v/rust-arc-gc.svg)](https://crates.io/crates/rust-arc-gc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Introduction

`rust-arc-gc` is a simple garbage collection (GC) implementation library designed for Rust, providing reference counting functionality similar to Rust's standard library `Arc`, but with added garbage collection capabilities. This library is particularly suitable for handling circular reference problems and applications requiring efficient memory management.

This library combines Rust's memory safety features with the convenience of garbage collection, offering a safe way to manage complex object graphs in Rust using a mark-and-sweep garbage collection algorithm.

## Core Features

- **Mark-and-Sweep Garbage Collection**: Uses a two-phase mark-and-sweep algorithm to detect and release objects that are no longer referenced, including circularly referenced objects
- **Dual Threshold Collection**: Supports both percentage-based and memory-based thresholds for triggering garbage collection
  - **Percentage Threshold**: Automatically triggers collection when attach operations exceed a configurable percentage of current object count
  - **Memory Threshold**: Triggers collection when allocated memory exceeds a specified byte limit
- **Type Safety**: Completely type-safe, leveraging Rust's type system for memory safety
- **Concurrency Safety**: All operations are thread-safe with atomic counters and proper synchronization
- **Weak Reference Support**: Provides `GCArcWeak` type for solving circular reference problems
- **Reference Tracking**: Implement the `GCTraceable` trait to make objects part of the garbage collection system
- **Memory Tracking**: Tracks allocated memory usage for better resource management

## Usage

### Installation

Use `cargo add rust-arc-gc` to add the library to your project.

## API Reference

### GC

#### Constructor Methods
- `GC::new()` - Create a new garbage collector instance with default 20% percentage threshold
- `GC::new_with_percentage(percentage)` - Create a garbage collector with custom percentage threshold (e.g., 30 for 30%)
- `GC::new_with_memory_threshold(memory_threshold)` - Create a garbage collector with memory threshold in bytes
- `GC::new_with_thresholds(percentage, memory_threshold)` - Create a garbage collector with both percentage and memory thresholds

#### Object Management Methods
- `gc.attach(obj)` - Add an object to the garbage collector's tracking scope (may trigger automatic collection)
- `gc.detach(obj)` - Remove an object from garbage collector tracking, returns `true` if object was found and removed
- `gc.create(obj)` - Create a new object and automatically add it to the garbage collector
- `gc.collect()` - Manually perform mark-and-sweep garbage collection

#### Information Methods
- `gc.object_count()` - Return the current number of objects managed by the garbage collector
- `gc.get_all()` - Return a vector of all objects currently managed by the garbage collector
- `gc.allocated_memory()` - Get the current estimated allocated memory in bytes
- `gc.memory_threshold()` - Get the current memory threshold setting
- `gc.set_memory_threshold(threshold)` - Set or update the memory threshold (None to disable)

#### Collection Triggering

The garbage collector uses multiple strategies to decide when to trigger collection:

- **Percentage Threshold**: Triggers when `attach_count >= current_objects * (percentage / 100)`
- **Memory Threshold**: Triggers when `allocated_memory >= memory_threshold` (if set)
- **Manual Triggering**: Always available via `collect()` method

Both thresholds (if configured) work independently - collection triggers when either condition is met.

### GCArc

- `GCArc::new(obj)` - Create a new reference-counted object
- `arc.as_ref()` - Get an immutable reference to the object
- `arc.get_mut()` - Get a mutable reference to the object (panics if not unique)
- `arc.try_as_mut()` - Try to get a mutable reference, returns `Option<&mut T>`
- `arc.as_weak()` - Create a weak reference to the object
- `arc.strong_ref()` - Get the current strong reference count
- `arc.weak_ref()` - Get the current weak reference count

### GCTraceable

A trait that must be implemented to allow the garbage collector to track object references:

```rust
pub trait GCTraceable<T: GCTraceable<T> + 'static> {
    /// Collects all reachable objects and adds them to the provided queue.
    /// This method is called during the mark phase of garbage collection
    /// to traverse the object graph.
    fn collect(&self, queue: &mut VecDeque<GCArcWeak<T>>);
}
```

**Implementation Guidelines:**
- Add any `GCArcWeak<T>` references held by your object to the queue
- This enables the garbage collector to traverse your object's references during the mark phase
- For objects with no references to other GC objects, an empty implementation is sufficient

### GCArcWeak

- `GCArcWeak::upgrade()` - Upgrade a weak reference to a strong reference, returning `None` if the object has been collected
- `GCArcWeak::is_valid()` - Check if the weak reference is valid (i.e., the object has not been collected)
- `weak.strong_ref()` - Get the current strong reference count
- `weak.weak_ref()` - Get the current weak reference count

## Usage Example

```rust
use arc_gc::{GC, GCArc, GCArcWeak, GCTraceable};
use std::collections::VecDeque;
use std::cell::RefCell;

// Define a node structure with potential circular references
struct Node {
    value: i32,
    children: RefCell<Vec<GCArcWeak<Node>>>,
}

impl GCTraceable<Node> for Node {
    fn collect(&self, queue: &mut VecDeque<GCArcWeak<Node>>) {
        // Add all child references to the collection queue
        if let Ok(children) = self.children.try_borrow() {
            for child in children.iter() {
                queue.push_back(child.clone());
            }
        }
    }
}

fn main() {
    let mut gc = GC::new_with_percentage(25); // 25% threshold
    
    // Create nodes
    let node1 = gc.create(Node {
        value: 1,
        children: RefCell::new(Vec::new()),
    });
    
    let node2 = gc.create(Node {
        value: 2,
        children: RefCell::new(Vec::new()),
    });
    
    // Create circular reference
    node1.as_ref().children.borrow_mut().push(node2.as_weak());
    node2.as_ref().children.borrow_mut().push(node1.as_weak());
    
    println!("Objects before collection: {}", gc.object_count());
    
    // Drop strong references
    drop(node1);
    drop(node2);
    
    // Manually trigger collection to clean up circular references
    gc.collect();
    
    println!("Objects after collection: {}", gc.object_count());
}
```

## Performance Considerations

The dual-threshold collection system provides flexible memory management:

### Threshold Configuration
- **Percentage Threshold (default: 20%)**:
  - Lower percentages (10-15%): More frequent collection, lower memory usage, higher CPU overhead
  - Higher percentages (30-50%): Less frequent collection, potentially higher memory usage, lower CPU overhead
- **Memory Threshold**: Set absolute memory limits for memory-constrained environments
- **Combined Thresholds**: Use both for fine-tuned control

### Collection Algorithm
- **Mark-and-Sweep**: Two-phase algorithm ensuring complete cycle detection
- **Root Detection**: Identifies objects with external references as collection roots
- **Thread Safety**: Atomic operations minimize locking overhead
- **Memory Tracking**: Estimates memory usage for threshold-based collection

### Optimization Tips
- Use `try_as_mut()` instead of `get_mut()` when mutation might fail
- Prefer `as_weak()` for non-owning references to prevent cycles
- Consider memory thresholds for applications with predictable memory patterns
- Monitor `allocated_memory()` and `object_count()` for tuning thresholds

## Limitations and Future Plans

### Current Limitations
- **Collection Algorithm**: Uses mark-and-sweep which may cause brief pauses during collection
- **Memory Estimation**: Object size estimation is approximate and may not account for all heap allocations
- **Single-threaded Collection**: Collection process is not parallelized

### Future Enhancements
- **Incremental Collection**: Reduce pause times by spreading collection work across multiple cycles
- **Generational Collection**: Optimize for typical allocation patterns with generational hypothesis
- **Parallel Collection**: Utilize multiple threads during mark and sweep phases
- **Adaptive Thresholds**: Automatically adjust thresholds based on allocation patterns and performance metrics
- **Enhanced Debugging**: Add memory usage statistics, collection timing, and object lifecycle tracking
- **Custom Allocators**: Integration with custom memory allocators for better memory tracking

## License

This project is licensed under the MIT License.