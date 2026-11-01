use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type Slot<T> = OnceLock<Arc<T>>;
type Values<K> = HashMap<(TypeId, K), Arc<dyn Any + Send + Sync>>;

pub(super) struct AnalysisCache<K = usize>(Mutex<Values<K>>);

impl<K> Default for AnalysisCache<K> {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl<K: std::hash::Hash + Eq> AnalysisCache<K> {
    pub(super) fn get<T: Send + Sync + 'static>(
        &self,
        key: K,
        initialize: impl FnOnce() -> T,
    ) -> Arc<T> {
        let slot = {
            let mut values = self.0.lock().unwrap();
            values
                .entry((TypeId::of::<T>(), key))
                .or_insert_with(|| Arc::new(Slot::<T>::new()))
                .clone()
                .downcast::<Slot<T>>()
                .unwrap()
        };
        slot.get_or_init(|| Arc::new(initialize())).clone()
    }
}

impl<K> std::fmt::Debug for AnalysisCache<K> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnalysisCache")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn each_typed_key_is_initialized_once_even_with_concurrent_queries() {
        let cache = AnalysisCache::default();
        let calls = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    assert_eq!(*cache.get(7, || calls.fetch_add(1, Ordering::SeqCst)), 0);
                });
            }
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(*cache.get(8, || 42usize), 42);
        assert_eq!(*cache.get(7, || "different type"), "different type");
        assert_eq!(*cache.get(9, || *cache.get(8, || 0usize)), 42);
    }
}
