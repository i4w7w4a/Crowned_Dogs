//! Inspect the source material atlas before authoring relief.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::{fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args().nth(1).unwrap_or(".".into());
    let root = Path::new(&root);
    let out = root.join("target/pom-materials");
    fs::create_dir_all(&out)?;
    for model in ["tripo-vest", "tripo-helmet", "varyag"] {
        let wrapper: Value = serde_json::from_slice(&fs::read(
            root.join(format!("apps/web/public/preview/{model}.json")),
        )?)?;
        let bytes = STANDARD.decode(wrapper["data"].as_str().unwrap())?;
        let n = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
        let doc: Value = serde_json::from_slice(&bytes[20..20 + n])?;
        let bin = &bytes[28 + n..];
        for (i, material) in doc["materials"].as_array().unwrap().iter().enumerate() {
            for (label, path) in [
                (
                    "color",
                    &material["pbrMetallicRoughness"]["baseColorTexture"],
                ),
                ("normal", &material["normalTexture"]),
            ] {
                let tex = &doc["textures"][path["index"].as_u64().unwrap() as usize];
                let image_index = tex["source"]
                    .as_u64()
                    .or(tex["extensions"]["EXT_texture_webp"]["source"].as_u64())
                    .unwrap() as usize;
                let view = &doc["bufferViews"]
                    [doc["images"][image_index]["bufferView"].as_u64().unwrap() as usize];
                let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
                let len = view["byteLength"].as_u64().unwrap() as usize;
                let image = image::load_from_memory(&bin[offset..offset + len])?;
                image
                    .resize(1024, 1024, image::imageops::FilterType::Triangle)
                    .save(out.join(format!("{model}-{i}-{label}.png")))?;
            }
        }
    }
    Ok(())
}
