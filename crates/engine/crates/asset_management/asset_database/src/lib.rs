use bevy_ecs::resource::Resource;
use shared::TextureKey;
use slotmap::SlotMap;
use uuid::Uuid;

#[derive(Resource)]
pub struct AssetDatabase {
    pub textures: SlotMap<TextureKey, Uuid>,
    //pub models: SlotMap<TextureKey, Uuid>,
    //pub materials: SlotMap<TextureKey, Uuid>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        AssetDatabase {
            textures: SlotMap::with_key(),
        }
    }
}
