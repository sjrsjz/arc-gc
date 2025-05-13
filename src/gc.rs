use std::sync::Mutex;

use crate::gc_ref::{GCArc, GCRef};

pub struct GC {
    gc_refs: Mutex<Vec<GCArc>>,
}

#[allow(dead_code)]
impl GC {
    pub fn new() -> Self {
        Self {
            gc_refs: Mutex::new(Vec::new()),
        }
    }

    pub fn attach(&mut self, gc_arc: GCArc) {
        let mut gc_refs = self.gc_refs.lock().unwrap();
        gc_refs.push(gc_arc);
    }

    pub fn collect(&mut self) {
        // Implement garbage collection logic here
        // For example, iterate over weak references and clean up if necessary
        let mut weak_refs = self.gc_refs.lock().unwrap();

        for r in weak_refs.iter_mut() {
            r.unmark();
        }

        let mut roots: Vec<&mut GCArc> = weak_refs
            .iter_mut()
            .filter(
                |r| r.strong_ref() > 1, // Assuming strong_ref() > 1 means it's a root
            )
            .collect();

        for r in roots.iter_mut() {
            r.mark_and_visit();
        }

        let retained: Vec<GCArc> = weak_refs
            .iter_mut()
            .filter(|r| r.is_marked())
            .map(|r| r.clone())
            .collect();

        weak_refs.clear();
        weak_refs.extend(retained);
    }

    pub fn object_count(&self) -> usize {
        return self.gc_refs.lock().unwrap().len();
    }
}
