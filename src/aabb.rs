use crate::Vec3;

/// Axis-aligned bounding box
#[derive(Copy, Clone, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3
}

pub trait Bounded {
    fn aabb(&self) -> Aabb;
}

impl Aabb {
    pub fn with_bounds(min: Vec3, max: Vec3) -> Self {
        Self {min, max}
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn centroid(&self) -> Vec3 {
        self.min + (self.size() / 2.0)
    }
}