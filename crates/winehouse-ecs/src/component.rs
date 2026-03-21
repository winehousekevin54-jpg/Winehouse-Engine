use std::any::TypeId;

/// Unique identifier for a component type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComponentId(TypeId);

impl ComponentId {
    pub fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>())
    }
}

/// Trait that all ECS components must implement.
pub trait Component: 'static + Send + Sync {}

// Blanket implementation: any type that is 'static + Send + Sync is a Component.
impl<T: 'static + Send + Sync> Component for T {}
