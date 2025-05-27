# Rust Arc GC (rust-arc-gc)

[![Crates.io](https://img.shields.io/crates/v/rust-arc-gc.svg)](https://crates.io/crates/rust-arc-gc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Introduction

`rust-arc-gc` is a simple garbage collection (GC) implementation library designed for Rust, providing reference counting functionality similar to Rust's standard library `Arc`, but with added garbage collection capabilities. This library is particularly suitable for handling circular reference problems and applications requiring efficient memory management.

This library combines Rust's memory safety features with the convenience of garbage collection, offering a safe way to manage complex object graphs in Rust.

## Core Features

- **Garbage Collection**: Can detect and release objects that are no longer referenced, including circularly referenced objects
- **Heuristic Collection**: Automatically triggers garbage collection when attach operations exceed a configurable percentage threshold
- **Type Safety**: Completely type-safe, no need for `unsafe` code (except within the library itself)
- **Concurrency Safety**: All operations are thread-safe with atomic counters
- **Weak Reference Support**: Provides `GCArcWeak` type for solving circular reference problems
- **Reference Tracking**: Implement the `GCTraceable` trait to make objects part of garbage collection

## Usage

### Installation

Use `cargo add rust-arc-gc` to add the library to your project.

## API Reference

### GC

- `GC::new()` - Create a new garbage collector instance with default 20% heuristic threshold
- `GC::new_with_percentage(percentage)` - Create a garbage collector with custom heuristic threshold (e.g., 30 for 30%)
- `gc.attach(obj)` - Add an object to the garbage collector's tracking scope (may trigger automatic collection)
- `gc.collect()` - Manually perform garbage collection
- `gc.object_count()` - Return the current number of objects managed by the garbage collector
- `gc.create(obj)` - Create a new object and add it to the garbage collector

#### Heuristic Collection

The garbage collector uses an atomic counter to track attach operations and automatically triggers collection when the number of attach operations exceeds a configurable percentage of the current object count. This helps maintain memory efficiency without requiring manual collection calls.

- Default threshold: 20% (triggers when attach_count >= current_objects * 0.2)
- Configurable via `new_with_percentage()`
- Counter resets after each collection cycle
- Thread-safe atomic operations

### GCArc

- `GCArc::new(obj)` - Create a new reference-counted object
- `arc.as_ref()` - Get a reference to the object
- `arc.as_mut()` - Get a mutable reference to the object  
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

## Performance Considerations

The heuristic collection system is designed to balance memory usage and performance:

- **Automatic Management**: Reduces the need for manual `collect()` calls
- **Configurable Thresholds**: Adjust collection frequency based on application needs
- **Atomic Operations**: Minimal overhead for tracking attach operations
- **Thread Safety**: All operations are safe for concurrent use

For applications with predictable allocation patterns, consider adjusting the percentage threshold:
- Lower percentages (10-15%): More frequent collection, lower memory usage
- Higher percentages (30-50%): Less frequent collection, potentially higher memory usage

## Limitations and Future Plans

- The current implementation uses a mark-sweep garbage collection algorithm; generational collection may be added in the future
- Performance optimization: reduce pause time during garbage collection
- Add richer debugging tools and memory usage statistics
- Consider adaptive thresholds based on allocation patterns

## License

This project is licensed under the MIT License.