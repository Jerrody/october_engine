use asset_database::AssetDatabase;
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
};
use shared::AssetMetadata;

#[derive(Resource)]
pub struct Loader {
    pub collected_meta_files: Vec<AssetMetadata>,
}

impl Loader {
    pub fn new() -> Self {
        Self {
            collected_meta_files: Vec::new(),
        }
    }
}

pub fn load_assets(loader: Res<Loader>, mut asset_database: ResMut<AssetDatabase>) {}
