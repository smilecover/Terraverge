//! `net.minecraft.client.resources.model.cuboid` 包。

mod cuboid_face;
mod cuboid_model;
mod cuboid_model_element;
mod cuboid_rotation;
mod face_bakery;

pub use cuboid_face::{CuboidFace, Uvs};
pub use cuboid_model::{CuboidModel, UnbakedCuboidGeometry};
pub use cuboid_model_element::CuboidModelElement;
pub use cuboid_rotation::CuboidRotation;
pub use face_bakery::FaceBakery;
