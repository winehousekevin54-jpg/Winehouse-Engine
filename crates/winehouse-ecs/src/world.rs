use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::entity::Entity;

/// The ECS World holds all entities and their components.
pub struct World {
    next_index: u32,
    generations: Vec<u32>,
    free_indices: Vec<u32>,
    components: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_index: 0,
            generations: Vec::new(),
            free_indices: Vec::new(),
            components: HashMap::new(),
        }
    }

    /// Spawn a new entity and return its handle.
    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free_indices.pop() {
            let generation = self.generations[index as usize];
            Entity::new(index, generation)
        } else {
            let index = self.next_index;
            self.next_index += 1;
            self.generations.push(0);
            Entity::new(index, 0)
        }
    }

    /// Despawn an entity, freeing its index for reuse.
    pub fn despawn(&mut self, entity: Entity) {
        let index = entity.index() as usize;
        if index < self.generations.len() && self.generations[index] == entity.generation() {
            self.generations[index] += 1;
            self.free_indices.push(entity.index());
            // Remove all components for this entity
            for storage in self.components.values_mut() {
                if let Some(map) = storage.downcast_mut::<HashMap<u32, Box<dyn Any>>>() {
                    map.remove(&entity.index());
                }
            }
        }
    }

    /// Check if an entity is still alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        let index = entity.index() as usize;
        index < self.generations.len() && self.generations[index] == entity.generation()
    }

    /// Insert a component for an entity.
    pub fn insert<T: 'static + Send + Sync>(&mut self, entity: Entity, component: T) {
        let storage = self
            .components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<u32, Box<dyn Any>>::new()));
        let map = storage.downcast_mut::<HashMap<u32, Box<dyn Any>>>().unwrap();
        map.insert(entity.index(), Box::new(component));
    }

    /// Get an immutable reference to a component for an entity.
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|storage| storage.downcast_ref::<HashMap<u32, Box<dyn Any>>>())
            .and_then(|map| map.get(&entity.index()))
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Get a mutable reference to a component for an entity.
    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components
            .get_mut(&TypeId::of::<T>())
            .and_then(|storage| storage.downcast_mut::<HashMap<u32, Box<dyn Any>>>())
            .and_then(|map| map.get_mut(&entity.index()))
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Get the number of active entities.
    pub fn entity_count(&self) -> usize {
        self.next_index as usize - self.free_indices.len()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_and_despawn() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        assert_eq!(world.entity_count(), 2);
        assert!(world.is_alive(e1));
        assert!(world.is_alive(e2));

        world.despawn(e1);
        assert!(!world.is_alive(e1));
        assert_eq!(world.entity_count(), 1);

        // Recycled index gets new generation
        let e3 = world.spawn();
        assert_eq!(e3.index(), e1.index());
        assert_ne!(e3.generation(), e1.generation());
    }

    #[test]
    fn insert_and_get_component() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, 42u32);
        world.insert(entity, String::from("hello"));

        assert_eq!(world.get::<u32>(entity), Some(&42));
        assert_eq!(world.get::<String>(entity), Some(&String::from("hello")));
        assert_eq!(world.get::<f64>(entity), None);
    }
}
