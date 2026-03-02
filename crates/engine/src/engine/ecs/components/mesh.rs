use bevy_ecs::component::Component;

use crate::engine::{
    components::local_transform::LocalTransform,
    ecs::{Vertex, materials_pool::MaterialReference, mesh_buffers_pool::MeshBufferReference},
};

#[derive(Component)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Component, Clone, Copy)]
#[require(LocalTransform)]
pub struct Mesh {
    pub(crate) mesh_buffer_reference: MeshBufferReference,
    pub(crate) material_reference: MaterialReference,
}
