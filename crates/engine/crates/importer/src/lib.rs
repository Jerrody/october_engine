use std::path::PathBuf;

use asset_importer::{Matrix4x4, node::Node, postprocess::PostProcessSteps};
use bevy_ecs::{resource::Resource, system::ResMut};
use bytemuck::{Pod, Zeroable};
use math::*;
use meshopt::*;
use padding_struct::padding_struct;
use walkdir::WalkDir;

type ModelLoader = asset_importer::Importer;
type Uuid = String;

struct NodeData {
    pub name: String,
    pub index: usize,
    pub parent_index: Option<usize>,
    pub matrix: Mat4,
    pub mesh_indices: Vec<usize>,
}

#[repr(C)]
#[padding_struct]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Meshlet {
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
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

pub struct Material {
    pub ptr_data: DeviceAddress,
    pub state: MaterialState,
}

#[repr(C)]
#[padding_struct]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

impl NodeData {
    pub fn new(
        name: String,
        index: usize,
        parent_index: Option<usize>,
        transformation: Matrix4x4,
        mesh_indices: Vec<usize>,
    ) -> Self {
        let matrix = Self::get_matrix(transformation);

        Self {
            name,
            index,
            parent_index,
            matrix,
            mesh_indices,
        }
    }

    pub fn get_matrix(transformation: Matrix4x4) -> Mat4 {
        math::Mat4 {
            x_axis: Vec4::new(
                transformation.x_axis.x,
                transformation.x_axis.y,
                transformation.x_axis.z,
                transformation.x_axis.w,
            ),
            y_axis: Vec4::new(
                transformation.y_axis.x,
                transformation.y_axis.y,
                transformation.y_axis.z,
                transformation.y_axis.w,
            ),
            z_axis: Vec4::new(
                transformation.z_axis.x,
                transformation.z_axis.y,
                transformation.z_axis.z,
                transformation.z_axis.w,
            ),
            w_axis: Vec4::new(
                transformation.w_axis.x,
                transformation.w_axis.y,
                transformation.w_axis.z,
                transformation.w_axis.w,
            ),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ModelAssetMetadata {
    name: String,
    path_buf: PathBuf,
    textures: Vec<Uuid>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct TextureAssetMetadata {
    uuid: Uuid,
    name: String,
    path_buf: Option<PathBuf>,
}

pub struct MaterialAssetMetadata {
    uuid: Uuid,
    name: Option<String>,
    path_buf: Option<PathBuf>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum AssetMetadata {
    Model(ModelAssetMetadata),
    Texture(TextureAssetMetadata),
}

pub struct Serializers {
    pub ron_pretty_config: ron::ser::PrettyConfig,
}

impl Serializers {
    pub fn new() -> Self {
        let ron_pretty_config = ron::ser::PrettyConfig::new()
            .depth_limit(2)
            .indentor("    ".to_string());

        Self { ron_pretty_config }
    }
}

#[derive(Clone)]
pub struct BaseAssetEntry {
    pub name: String,
    pub path_buf: PathBuf,
}

#[derive(Clone)]
pub struct ModelEntry {
    pub entry: BaseAssetEntry,
}

// TODO: Not sure if it's a good naming.
#[derive(Clone)]
pub enum AssetEntry {
    Model(ModelEntry),
}

#[derive(Resource)]
pub struct Importer {
    model_importer: ModelLoader,
    asset_folder_path_buffer: PathBuf,
    assets_to_serialize: Vec<PathBuf>,
    serializers: Serializers,
    meta_files: Vec<AssetMetadata>,
    assets_entries: Vec<AssetEntry>,
}

impl Importer {
    pub fn new() -> Self {
        Self {
            model_importer: ModelLoader::new(),
            asset_folder_path_buffer: Self::get_assets_folder_path_buffer(),
            assets_to_serialize: Default::default(),
            serializers: Serializers::new(),
            meta_files: Vec::new(),
            assets_entries: Vec::new(),
        }
    }

    fn get_assets_folder_path_buffer() -> PathBuf {
        let mut exe_path = std::env::current_exe().unwrap();

        exe_path.pop();
        exe_path.pop();
        exe_path.pop();
        exe_path.push("assets");

        exe_path
    }
}

pub fn collect_assets_to_serialize_system(mut importer: ResMut<Importer>) {
    importer.assets_to_serialize.clear();
    importer.meta_files.clear();

    let assets_folder_path = importer.asset_folder_path_buffer.as_path();

    for entry in WalkDir::new(assets_folder_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if entry
                .path()
                .extension()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".meta")
            {
                let meta_file = ron::de::from_reader::<std::fs::File, AssetMetadata>(
                    std::fs::File::open(entry.path()).unwrap(),
                )
                .unwrap();

                importer.meta_files.push(meta_file);
            } else {
                importer
                    .assets_to_serialize
                    .push(entry.path().to_path_buf());
            }
        }
    }
}

pub fn resolve_assets_entries_system(mut importer: ResMut<Importer>) {
    let mut asset_entries = Vec::with_capacity(importer.assets_to_serialize.len());

    importer
        .assets_to_serialize
        .drain(..)
        .for_each(|asset_to_resolve| {
            let file_name = asset_to_resolve
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();

            match asset_to_resolve
                .extension()
                .unwrap()
                .to_str()
                .unwrap_or_default()
            {
                "glb" | "gltf" | "obj" | "fbx" => {
                    asset_entries.push(AssetEntry::Model(ModelEntry {
                        entry: BaseAssetEntry {
                            name: file_name,
                            path_buf: asset_to_resolve.clone(),
                        },
                    }));
                }
                _ => (),
            }
        });

    importer.assets_entries.clear();
    importer.assets_entries.append(&mut asset_entries);
}

pub fn check_if_asset_is_serialized_system(mut importer: ResMut<Importer>) {
    let meta_files = importer.meta_files.to_vec();

    importer.assets_entries.retain(|asset_entry| {
        let name = match asset_entry {
            AssetEntry::Model(model_entry) => model_entry.entry.name.as_str(),
        };
        let path = match asset_entry {
            AssetEntry::Model(model_entry) => model_entry.entry.path_buf.as_path(),
        };

        !meta_files.iter().any(|meta_file| {
            let meta_name = match meta_file {
                AssetMetadata::Model(model_asset) => model_asset.name.as_str(),
                AssetMetadata::Texture(texture_asset_metadata) => todo!(),
            };
            let meta_path = match meta_file {
                AssetMetadata::Model(model_asset) => model_asset.path_buf.as_path(),
                AssetMetadata::Texture(texture_asset_metadata) => todo!(),
            };

            name.eq(meta_name) && path.eq(meta_path)
        })
    });
}

pub fn serialize_unserialized_assets_system(mut importer: ResMut<Importer>) {
    let mut assets_entries = importer.assets_entries.to_vec();

    assets_entries
        .drain(..)
        .for_each(|asset_entry| match asset_entry {
            AssetEntry::Model(model_entry) => serialize_model_asset(&mut importer, &model_entry),
        });
}

// TODO: Currently, we serialize and model, and textures, and materials in the same pass, later, need to separate them.
fn serialize_model_asset(importer: &mut Importer, model_entry: &ModelEntry) {
    let model_name = model_entry.entry.name.as_str();
    let model_path = model_entry.entry.path_buf.as_path();

    let scene = importer
        .model_importer
        .read_file(model_path)
        .with_post_process(PostProcessSteps::MAX_QUALITY | PostProcessSteps::FLIP_UVS)
        .import()
        .unwrap();

    let mut nodes = Vec::new();

    let root_node_index = Default::default();
    let root_node = scene.root_node().unwrap();

    nodes.push(NodeData::new(
        root_node.name(),
        root_node_index,
        None,
        root_node.transformation(),
        get_mesh_indices(&root_node, root_node.num_meshes()),
    ));

    let mut stack: Vec<(Node, usize)> = Vec::new();
    stack.push((root_node, root_node_index));

    loop {
        while let Some((parent_node, parent_index_in_array)) = stack.pop() {
            for child_index in (0..parent_node.num_children()).rev() {
                let child_node = parent_node.child(child_index).unwrap();

                let child_index_in_array = nodes.len();
                stack.push((child_node.clone(), child_index_in_array));

                nodes.push(NodeData::new(
                    child_node.name(),
                    child_index_in_array,
                    Some(parent_index_in_array),
                    child_node.transformation(),
                    get_mesh_indices(&child_node, child_node.num_meshes()),
                ));
            }
        }

        if stack.len() == Default::default() {
            break;
        }
    }

    for node_data in nodes.into_iter() {
        if node_data.mesh_indices.len() > Default::default() {
            let mut mesh_name: String;
            for &mesh_index in node_data.mesh_indices.iter() {
                texture_reference = renderer_resources.fallback_texture_reference;
                let mesh = scene.mesh(mesh_index).unwrap();

                let material_index = mesh.material_index();
                let material_reference: MaterialReference;
                if let std::collections::hash_map::Entry::Vacant(e) =
                    uploaded_materials.entry(material_index)
                {
                    let material = scene.material(material_index).unwrap();

                    let alpha_mode = std::str::from_utf8(
                        material
                            .get_property_raw_ref(c"$mat.gltf.alphaMode", None, 0)
                            .unwrap(),
                    )
                    .unwrap();
                    let mut material_type = MaterialType::Opaque;
                    if alpha_mode.contains("BLEND") {
                        material_type = MaterialType::Transparent;
                    }

                    try_upload_texture(
                        &vulkan_context,
                        &renderer_context_resource,
                        &mut textures_pool,
                        &mut buffers_pool,
                        &mut descriptor_set_handle,
                        &scene,
                        &mut uploaded_textures,
                        material.clone(),
                        &mut texture_reference,
                        load_model_event.path.file_stem().unwrap().to_str().unwrap(),
                    );

                    let base_color_raw = material.base_color().unwrap();
                    let base_color = Vec4::new(
                        base_color_raw.x,
                        base_color_raw.y,
                        base_color_raw.z,
                        base_color_raw.w,
                    );

                    let metallic_value = material.metallic_factor().unwrap_or(0.0);
                    let roughness_value = material.roughness_factor().unwrap_or(0.0);
                    let albedo_texture_index = texture_reference.get_index();
                    let metallic_texture_index =
                        renderer_resources.fallback_texture_reference.get_index();
                    let roughness_texture_index =
                        renderer_resources.fallback_texture_reference.get_index();

                    let material_data = MaterialData {
                        material_properties: MaterialProperties::new(
                            base_color,
                            metallic_value,
                            roughness_value,
                        ),
                        material_textures: MaterialTextures::new(
                            albedo_texture_index,
                            metallic_texture_index,
                            roughness_texture_index,
                        ),
                        sampler_index: Default::default(),
                    };

                    material_reference = materials_pool.write_material(
                        bytemuck::bytes_of(&material_data),
                        MaterialState { material_type },
                    );
                    e.insert(material_reference);
                } else {
                    material_reference = *uploaded_materials.get(&material_index).unwrap();
                }

                if let std::collections::hash_map::Entry::Vacant(e) =
                    uploaded_mesh_buffers.entry(mesh_index)
                {
                    let mesh = scene.mesh(mesh_index).unwrap();
                    mesh_name = mesh.name();

                    let mut indices = Vec::with_capacity(mesh.faces().len() * 3);

                    for face in mesh.faces() {
                        for index in face.indices() {
                            indices.push(*index);
                        }
                    }

                    let positions: Vec<Vec3> = mesh
                        .vertices_iter()
                        .map(|v| Vec3::new(v.x, v.y, v.z))
                        .collect();
                    let colors: Vec<Vec3> = mesh
                        .vertex_colors(Default::default())
                        .map(|colors| {
                            colors
                                .iter()
                                .map(|color| Vec3::new(color.x, color.y, color.z))
                                .collect()
                        })
                        .unwrap_or_else(|| vec![Vec3::ZERO; positions.len()]);
                    let normals: Vec<Vec3> = mesh
                        .normals()
                        .map(|ns| ns.iter().map(|n| Vec3::new(n.x, n.y, n.z)).collect())
                        .unwrap_or_else(|| vec![Vec3::ZERO; positions.len()]);

                    let uvs: Vec<Vec2> = if mesh.has_texture_coords(0) {
                        mesh.texture_coords_iter(0)
                            .map(|uv| Vec2::new(uv.x, uv.y))
                            .collect()
                    } else {
                        vec![Vec2::ZERO; positions.len()]
                    };

                    let mut vertices = Vec::with_capacity(positions.len());
                    for i in 0..positions.len() {
                        vertices.push(Vertex {
                            position: positions[i].to_array(),
                            normal: normals[i].to_array(),
                            uv: uvs[i].to_array(),
                            color: colors[i].to_array(),
                            ..Default::default()
                        });
                    }

                    let remap = optimize_vertex_fetch_remap(&indices, vertices.len());
                    indices = remap_index_buffer(Some(&indices), vertices.len(), &remap);
                    vertices = remap_vertex_buffer(&vertices, vertices.len(), &remap);

                    let position_offset = std::mem::offset_of!(Vertex, position);
                    let vertex_stride = std::mem::size_of::<Vertex>();
                    let vertex_data = typed_to_bytes(&vertices);

                    let vertex_data_adapter =
                        VertexDataAdapter::new(vertex_data, vertex_stride, position_offset)
                            .unwrap();

                    optimize_vertex_cache_in_place(&mut indices, vertices.len());
                    let vertices = optimize_vertex_fetch(&mut indices, &vertices);

                    let (meshlets, vertex_indices, triangles) =
                        generate_meshlets(&indices, &vertex_data_adapter);

                    let vertex_buffer_reference = create_and_copy_to_buffer(
                        &mut buffers_pool,
                        vertices.as_ptr() as *const _,
                        vertices.len() * std::mem::size_of::<Vertex>(),
                        std::format!("{}_{}", mesh_name, name_of!(vertices)),
                    );
                    let vertex_indices_buffer_reference = create_and_copy_to_buffer(
                        &mut buffers_pool,
                        vertex_indices.as_ptr() as _,
                        vertex_indices.len() * std::mem::size_of::<u32>(),
                        std::format!("{}_{}", mesh_name, name_of!(vertex_indices)),
                    );
                    let meshlets_buffer_reference = create_and_copy_to_buffer(
                        &mut buffers_pool,
                        meshlets.as_ptr() as _,
                        meshlets.len() * std::mem::size_of::<Meshlet>(),
                        std::format!("{}_{}", mesh_name, name_of!(meshlets)),
                    );

                    let local_indices_buffer_reference = create_and_copy_to_buffer(
                        &mut buffers_pool,
                        triangles.as_ptr() as _,
                        triangles.len() * std::mem::size_of::<u8>(),
                        std::format!("{}_{}", mesh_name, name_of!(triangles)),
                    );

                    let mesh_data = MeshData { vertices, indices };

                    let mesh_buffer = MeshBuffer {
                        mesh_object_device_address: Default::default(),
                        vertex_buffer_reference,
                        vertex_indices_buffer_reference,
                        meshlets_buffer_reference,
                        local_indices_buffer_reference,
                        meshlets_count: meshlets.len(),
                        mesh_data,
                    };

                    mesh_buffer_reference = mesh_buffers_pool.insert_mesh_buffer(mesh_buffer);
                    mesh_buffers_to_upload.push(mesh_buffer_reference);

                    e.insert((mesh, mesh_buffer_reference));
                } else {
                    let already_uploaded_mesh = uploaded_mesh_buffers.get(&mesh_index).unwrap();
                    mesh_name = already_uploaded_mesh.0.name();
                    mesh_buffer_reference = already_uploaded_mesh.1;
                }

                spawn_event_record.name = mesh_name;
                spawn_event_record.parent_index = Some(node_data.index);
                spawn_event_record.material_reference = Some(material_reference);
                spawn_event_record.mesh_buffer_reference = Some(mesh_buffer_reference);
                spawn_event_record.transform = LocalTransform::IDENTITY;

                spawn_event.spawn_records.push(spawn_event_record.clone());
            }
        }
    }
}

fn get_mesh_indices(node: &Node, num_meshes: usize) -> Vec<usize> {
    let mut mesh_indices = Vec::with_capacity(num_meshes);
    if num_meshes > Default::default() {
        for mesh_index in node.mesh_indices() {
            mesh_indices.push(mesh_index);
        }
    }

    mesh_indices
}

fn generate_meshlets(
    indices: &[u32],
    vertices: &VertexDataAdapter,
) -> (Vec<Meshlet>, Vec<u32>, Vec<u8>) {
    let max_vertices = 64;
    let max_triangles = 64;
    let cone_weight = 0.0;

    let raw_meshlets = build_meshlets(indices, vertices, max_vertices, max_triangles, cone_weight);

    let mut meshlets = Vec::new();

    for raw_meshlet in raw_meshlets.meshlets.iter() {
        meshlets.push(Meshlet {
            vertex_offset: raw_meshlet.vertex_offset as _,
            triangle_offset: raw_meshlet.triangle_offset as _,
            vertex_count: raw_meshlet.vertex_count as _,
            triangle_count: raw_meshlet.triangle_count as _,
            ..Default::default()
        });
    }

    (meshlets, raw_meshlets.vertices, raw_meshlets.triangles)
}

/* fn try_upload_texture(
    vulkan_context: &VulkanContextResource,
    renderer_context: &RendererContext,
    textures_pool: &mut TexturesPool,
    buffers_pool: &mut BuffersPool,
    descriptor_set_handle: &mut DescriptorSetHandle,
    scene: &asset_importer::Scene,
    uploaded_textures: &mut HashMap<usize, TextureReference>,
    material: asset_importer::Material,
    texture_reference_to_use: &mut TextureReference,
    model_name: &str,
) {
    if material.texture_count(asset_importer::TextureType::BaseColor) > Default::default() {
        let texture_info = material
            .texture(asset_importer::TextureType::BaseColor, Default::default())
            .unwrap();
        let texture_index = texture_info.path[1..].parse::<usize>().unwrap();

        if let std::collections::hash_map::Entry::Vacant(e) = uploaded_textures.entry(texture_index)
        {
            let texture = scene.texture(texture_index).unwrap();
            let texture_name = texture
                .filename()
                .unwrap_or(std::format!("{model_name}_texture_{texture_index}"));

            let (texture_reference, texture_data) = try_to_load_cached_texture(
                textures_pool,
                model_name,
                texture.clone(),
                &texture_name,
            );
            *texture_reference_to_use = texture_reference;

            vulkan_context.transfer_data_to_image(
                textures_pool.get_image(texture_reference).unwrap(),
                buffers_pool,
                texture_data.as_ptr() as *const _,
                &renderer_context.upload_context,
                Some(texture_data.len()),
            );

            let descriptor_texture = DescriptorKind::SampledImage(DescriptorSampledImage {
                image_view: textures_pool
                    .get_image(texture_reference)
                    .unwrap()
                    .image_view,
                index: texture_reference.get_index(),
            });
            descriptor_set_handle.update_binding(buffers_pool, descriptor_texture);

            let texture_metadata = texture_reference.texture_metadata;
            println!(
                "Name: {} | Index: {} | Extent: {}x{}x{}",
                texture_name,
                texture_reference.get_index(),
                texture_metadata.width,
                texture_metadata.height,
                1,
            );

            e.insert(texture_reference);
        } else {
            *texture_reference_to_use = *uploaded_textures.get(&texture_index).unwrap();
        }
    }
}

fn try_to_load_cached_texture(
    textures_pool: &mut TexturesPool,
    model_name: &str,
    texture: asset_importer::Texture,
    texture_name: &str,
) -> (TextureReference, Vec<u8>) {
    let mut path = std::path::PathBuf::from("intermediate/textures/");
    path.push(model_name);
    std::fs::create_dir_all(&path).unwrap();

    path.push(String::from_str(texture_name).unwrap());
    let does_exist = std::fs::exists(&path).unwrap();

    let texture_reference: TextureReference;
    let mut texture_data: Vec<u8> = Vec::new();

    if does_exist {
        let texture = Ktx2Texture::from_file(&path).unwrap();
        let texture_metadata_raw: Vec<u8> =
            texture.get_metadata(stringify!(TextureMetadata)).unwrap();
        let texture_metadata = *bytemuck::from_bytes::<TextureMetadata>(&texture_metadata_raw);

        for mip_level_index in 0..texture_metadata.mip_levels_count {
            texture_data.extend_from_slice(texture.get_image_data(mip_level_index, 0, 0).unwrap());
        }

        let extent = Extent3D {
            width: texture_metadata.width,
            height: texture_metadata.height,
            depth: 1,
        };

        let (created_texture_reference, _) = textures_pool.create_texture(
            Some(&mut texture_data),
            true,
            Format::Bc1RgbSrgbBlock,
            extent,
            ImageUsageFlags::Sampled | ImageUsageFlags::TransferDst,
            true,
        );

        texture_reference = created_texture_reference;
    } else {
        let mut data = texture.data_bytes_ref().unwrap();

        let cursor = Cursor::new(&mut data);

        let image = ImageReader::new(cursor)
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        let extent = Extent3D {
            width: image.width(),
            height: image.height(),
            depth: 1,
        };
        let rgba_image = image.to_rgba8();
        let mut image_bytes = rgba_image.as_bytes().to_vec();

        let (created_texture_reference, ktx_texture) = textures_pool.create_texture(
            Some(&mut image_bytes),
            false,
            Format::Bc1RgbSrgbBlock,
            extent,
            ImageUsageFlags::Sampled | ImageUsageFlags::TransferDst,
            true,
        );
        texture_reference = created_texture_reference;

        let ktx_texture = ktx_texture.unwrap();
        for mip_level_index in 0..created_texture_reference.texture_metadata.mip_levels_count {
            texture_data
                .extend_from_slice(ktx_texture.get_image_data(mip_level_index, 0, 0).unwrap());
        }

        ktx_texture.write_to_file(path).unwrap();
    }

    (texture_reference, texture_data)
} */
