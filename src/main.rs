use gc_ref::{GCArc, GCArcWeak, GCRef, GCTraceable};

mod gc;
mod gc_ref;
mod gc_tests;
mod gc_advanced_tests;
#[cfg(test)]
mod gc_list_test;

struct GCList {
    value: i32,
    next: Option<GCArcWeak>,
}

impl Drop for GCList {
    fn drop(&mut self) {
        println!("Dropping GCList with value: {}", self.value);
    }
    
}

impl GCTraceable for GCList {
    fn visit(&self) {
        if let Some(ref next) = self.next {
            next.mark_and_visit();
        }
    }
}

fn main() {
    let mut gc = gc::GC::new();

    let list_head;
    {
        let list = GCArc::new(GCList {
            value: 1,
            next: None,
        });
        let mut current = list.clone();
        for i in 2..=10 {
            let new_node = GCArc::new(GCList {
                value: i,
                next: None,
            });
            current.downcast_mut::<GCList>().next = Some(new_node.as_weak());
            current = new_node.clone();
        }
        gc.attach(list.clone());
        list_head = list.as_weak();
    }

    gc.collect();


    println!("Head: {}", list_head.downcast::<GCList>().value);
    let mut current = list_head;
    while let Some(ref mut next) = current.downcast_mut::<GCList>().next {
        println!("Next: {}", next.downcast::<GCList>().value);
        current = next.clone();
    }
    gc.collect();
}
