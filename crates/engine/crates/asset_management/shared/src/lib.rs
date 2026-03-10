use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use math::Vec4;
use padding_struct::padding_struct;
use uuid::Uuid;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SerializedMesh {
    // NOTE: Vertices and Inddices baked by meshopt, can be issues with creating colliders, but need to check.
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub meshlets: Vec<Meshlet>,
    pub triangles: Vec<u8>,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct TextureMetadata {
    pub texture_format: u32,
    pub width: u32,
    pub height: u32,
    pub mip_levels_count: u32,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SerializedHierarchy {
    pub serialized_nodes: Vec<SerializedNode>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SerializedNode {
    pub name: String,
    pub parent_index: Option<usize>,
    pub matrix: [f32; 16],
    pub mesh_index: Option<usize>,
}

pub struct SerializedModelResult {
    pub serialized_model: SerializedModel,
    pub associated_texture_entries: Vec<TextureEntry>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SerializedModel {
    pub meshes: Vec<SerializedMesh>,
    pub hierarchy: SerializedHierarchy,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SerializedTexture {
    pub data: Vec<u8>,
}

#[repr(C)]
#[padding_struct]
#[derive(
    Default, Clone, Copy, Pod, Zeroable, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Meshlet {
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
}

#[repr(C)]
#[padding_struct]
#[derive(
    Default, Clone, Copy, Pod, Zeroable, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

#[derive(Default, Clone, Copy)]
#[repr(u8)]
pub enum MaterialType {
    #[default]
    Opaque,
    Transparent,
}

#[derive(Clone, Copy)]
pub struct MaterialState {
    pub material_type: MaterialType,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MaterialProperties {
    pub base_color: [f32; 4],
    pub metallic_value: f32,
    pub roughness_value: f32,
}

impl MaterialProperties {
    pub fn new(base_color: Vec4, metallic_value: f32, roughness_value: f32) -> Self {
        Self {
            base_color: base_color.to_array(),
            metallic_value,
            roughness_value,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MaterialTextures {
    pub albedo_texture_index: u32,
    pub metallic_texture_index: u32,
    pub roughness_texture_index: u32,
}

impl MaterialTextures {
    pub fn new(
        albedo_texture_index: u32,
        metallic_texture_index: u32,
        roughness_texture_index: u32,
    ) -> Self {
        Self {
            albedo_texture_index,
            metallic_texture_index,
            roughness_texture_index,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MaterialData {
    pub material_properties: MaterialProperties,
    pub material_textures: MaterialTextures,
    pub sampler_index: u32,
}

#[repr(C)]
#[padding_struct]
#[derive(Default, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct TextureInput {
    pub uuid: Uuid,
    pub offset: usize,
}

#[repr(C)]
#[padding_struct]
#[derive(Default, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct SerializedMaterial {
    pub size: usize,
    pub data: Vec<u8>,
    pub texture_inputs: Vec<TextureInput>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ModelAssetMetadata {
    pub uuid: Uuid,
    pub name: String,
    pub path_buf: PathBuf,
    //materials: Vec<Uuid>,
    // TODO: Temp comment1ing.
    //textures: Vec<Uuid>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct TextureAssetMetadata {
    pub uuid: Uuid,
    pub name: String,
    pub path_buf: PathBuf,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialAssetMetadata {
    pub uuid: Uuid,
    pub name: String,
    pub path_buf: PathBuf,
    pub textures: Vec<Uuid>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum AssetMetadata {
    Model(ModelAssetMetadata),
    Texture(TextureAssetMetadata),
    Material(MaterialAssetMetadata),
}

#[derive(Clone)]
pub struct BaseAssetEntry {
    pub name: String,
    pub extension: String,
    pub path_buf: PathBuf,
}

#[derive(Clone)]
pub struct ModelEntry {
    pub entry: BaseAssetEntry,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum TextureFormat {
    RGBA8,
    RGB8,
    Bc1,
    Bc3,
    Bc4,
    Bc5,
    Bc6H,
    Bc7,
}

#[derive(Clone)]
pub struct TextureEntry {
    pub entry: BaseAssetEntry,
    pub format: TextureFormat,
    pub associated_model: Option<ModelEntry>,
}

// TODO: Not sure if it's a good naming.
#[derive(Clone)]
pub enum AssetEntry {
    Model(ModelEntry),
    Texture(TextureEntry),
}

slotmap::new_key_type! {
    pub struct BufferKey;
    pub struct TextureKey;
    pub struct SamplerKey;
    pub struct MeshBufferKey;
    pub struct MeshDataKey;
    pub struct MaterialKey;
    pub struct AudioKey;
}
