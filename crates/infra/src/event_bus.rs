use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;

type Handler = Box<dyn Fn(&dyn Any) + Send + Sync>;

pub struct EventBus {
    inner: RwLock<HashMap<TypeId, Vec<Handler>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn subscribe<E: 'static + Send + Sync>(
        &self,
        handler: impl Fn(&E) + Send + Sync + 'static,
    ) {
        let mut map = self.inner.write();
        map.entry(TypeId::of::<E>())
            .or_default()
            .push(Box::new(move |any| {
                if let Some(e) = any.downcast_ref::<E>() {
                    handler(e);
                }
            }));
    }

    pub fn publish<E: 'static + Send + Sync>(&self, event: E) {
        if let Some(list) = self.inner.read().get(&TypeId::of::<E>()) {
            for h in list {
                h(&event);
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn test_event_bus() {
        let bus = Arc::new(EventBus::new());
        let c = Arc::new(AtomicUsize::new(0));
        {
            let bus = bus.clone();
            let c = c.clone();
            bus.subscribe(move |s: &String| {
                if s == "hi" {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
        bus.publish("hi".to_string());
        assert_eq!(c.load(Ordering::SeqCst), 1);
    }
}
