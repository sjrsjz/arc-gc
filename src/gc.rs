use std::sync::Mutex;

use crate::{
    arc::{GCArc, GCRef},
    traceable::GCTraceable,
};

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

    pub fn detach(&mut self, gc_arc: &GCArc) -> bool {
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

        let mut roots: Vec<&mut GCArc> = refs
            .iter_mut()
            .filter(
                |r| r.strong_ref() > 1, // Assuming strong_ref() > 1 means it's a root
            )
            .collect();

        for r in roots.iter_mut() {
            r.mark_and_visit();
        }

        let retained: Vec<GCArc> = refs
            .iter_mut()
            .filter(|r| r.is_marked())
            .map(|r| r.clone())
            .collect();

        refs.clear();
        refs.extend(retained);
    }

    pub fn object_count(&self) -> usize {
        return self.gc_refs.lock().unwrap().len();
    }

    pub fn get_all(&self) -> Vec<GCArc> {
        self.gc_refs.lock().unwrap().clone()
    }
    
    pub fn create<T: GCTraceable + 'static>(&mut self, obj: T) -> GCArc {
        let gc_arc = GCArc::new(obj);
        self.attach(gc_arc.clone());
        gc_arc
    }
}
