use std::path::PathBuf;

use bevy_ecs::{resource::Resource, system::ResMut};
use walkdir::WalkDir;

type ModelLoader = asset_importer::Importer;
type Uuid = String;

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct ModelAssetMetadata {
    name: String,
    path: PathBuf,
    textures: Vec<Uuid>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct TextureAssetMetadata {
    uuid: Uuid,
    name: String,
    path: Option<PathBuf>,
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

pub struct BaseAssetEntry {
    pub name: String,
    pub path: PathBuf,
}

pub struct ModelEntry {
    pub asset_entry: BaseAssetEntry,
}

// TODO: Not sure if it's a good naming.
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
    asset_entries: Vec<AssetEntry>,
}

impl Importer {
    pub fn new() -> Self {
        Self {
            model_importer: ModelLoader::new(),
            asset_folder_path_buffer: Self::get_assets_folder_path_buffer(),
            assets_to_serialize: Default::default(),
            serializers: Serializers::new(),
            meta_files: Vec::new(),
            asset_entries: Vec::new(),
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

pub fn collect_assets_to_serialize(mut importer: ResMut<Importer>) {
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

pub fn resolve_assets_entries(mut importer: ResMut<Importer>) {
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
                        asset_entry: BaseAssetEntry {
                            name: file_name,
                            path: asset_to_resolve.clone(),
                        },
                    }));
                }
                _ => (),
            }
        });

    importer.asset_entries.clear();
    importer.asset_entries.append(&mut asset_entries);
}

pub fn check_if_asset_is_serialized(mut importer: ResMut<Importer>) {
    let meta_files = importer.meta_files.to_vec();

    importer.asset_entries.retain(|asset_entry| {
        let name = match asset_entry {
            AssetEntry::Model(model_entry) => model_entry.asset_entry.name.as_str(),
        };
        let path = match asset_entry {
            AssetEntry::Model(model_entry) => model_entry.asset_entry.path.as_path(),
        };

        !meta_files.iter().any(|meta_file| {
            let meta_name = match meta_file {
                AssetMetadata::Model(model_asset) => model_asset.name.as_str(),
                AssetMetadata::Texture(texture_asset_metadata) => todo!(),
            };
            let meta_path = match meta_file {
                AssetMetadata::Model(model_asset) => model_asset.path.as_path(),
                AssetMetadata::Texture(texture_asset_metadata) => todo!(),
            };

            name.eq(meta_name) && path.eq(meta_path)
        })
    });
}
