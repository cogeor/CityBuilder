//! Resource — type-erased resource storage.
//!
//! Resources are singleton data accessible by systems. They are stored
//! in a type-map inside [`App`] and retrieved by type.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Marker trait for resources stored in the App.
pub trait Resource: Any + Send + Sync + 'static {}

/// Blanket impl: any `Send + Sync + 'static` type can be a Resource.
impl<T: Any + Send + Sync + 'static> Resource for T {}

/// Type-erased storage for resources, keyed by TypeId.
#[derive(Default)]
pub struct ResourceMap {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ResourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a resource, replacing any existing one of the same type.
    pub fn insert<R: Resource>(&mut self, resource: R) {
        self.map.insert(TypeId::of::<R>(), Box::new(resource));
    }

    /// Get a reference to a resource by type.
    pub fn get<R: Resource>(&self) -> Option<&R> {
        self.map
            .get(&TypeId::of::<R>())
            .and_then(|r| r.downcast_ref::<R>())
    }

    /// Get a mutable reference to a resource by type.
    pub fn get_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.map
            .get_mut(&TypeId::of::<R>())
            .and_then(|r| r.downcast_mut::<R>())
    }

    /// Check if a resource of this type exists.
    pub fn contains<R: Resource>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<R>())
    }

    /// Remove a resource by type, returning it if it existed.
    pub fn remove<R: Resource>(&mut self) -> Option<R> {
        self.map
            .remove(&TypeId::of::<R>())
            .and_then(|r| r.downcast::<R>().ok())
            .map(|r| *r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter(u32);

    #[test]
    fn insert_and_get() {
        let mut map = ResourceMap::new();
        map.insert(Counter(42));
        assert_eq!(map.get::<Counter>().unwrap().0, 42);
    }

    #[test]
    fn get_mut() {
        let mut map = ResourceMap::new();
        map.insert(Counter(0));
        map.get_mut::<Counter>().unwrap().0 = 99;
        assert_eq!(map.get::<Counter>().unwrap().0, 99);
    }

    #[test]
    fn remove() {
        let mut map = ResourceMap::new();
        map.insert(Counter(7));
        let c = map.remove::<Counter>().unwrap();
        assert_eq!(c.0, 7);
        assert!(!map.contains::<Counter>());
    }
}
