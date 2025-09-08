use egui::Color32;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize_color32<S>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let [r, g, b, a] = color.to_array();
    [r, g, b, a].serialize(serializer)
}

pub fn deserialize_color32<'de, D>(deserializer: D) -> Result<Color32, D::Error>
where
    D: Deserializer<'de>,
{
    let [r, g, b, a] = <[u8; 4]>::deserialize(deserializer)?;
    Ok(Color32::from_rgba_unmultiplied(r, g, b, a))
}
