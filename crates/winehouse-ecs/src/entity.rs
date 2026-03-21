/// A unique identifier for an entity in the ECS world.
/// Uses a generational index: lower 32 bits are the index, upper 32 bits are the generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    id: u64,
}

impl Entity {
    pub fn new(index: u32, generation: u32) -> Self {
        Self {
            id: (generation as u64) << 32 | index as u64,
        }
    }

    pub fn index(self) -> u32 {
        self.id as u32
    }

    pub fn generation(self) -> u32 {
        (self.id >> 32) as u32
    }

    pub fn id(self) -> u64 {
        self.id
    }
}
