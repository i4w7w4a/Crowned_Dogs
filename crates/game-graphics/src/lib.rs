//! Platform-independent procedural graphics. Three.js only uploads these results.
mod carry;
mod stage;
mod surface;
mod texture;
mod view;

use serde_json::{json, Value};

pub fn texture(kind: &str, size: u32) -> Result<Vec<u8>, String> {
    if !["steel-color", "steel-roughness", "steel-relief", "contact"].contains(&kind) {
        return Err(format!("Unknown procedural texture: {kind}"));
    }
    if !(32..=2048).contains(&size) {
        return Err("Texture size must be between 32 and 2048".into());
    }
    Ok(texture::generate(kind, size as usize))
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    match method {
        "graphics.fittingCamera" => view::fitting_camera(&args),
        "graphics.fittingAim" => view::fitting_aim(&args),
        "graphics.fittingRotation" => Ok(json!((args["angle"].as_f64().unwrap_or(0.)
            + args["delta"].as_f64().unwrap_or(0.))
        .rem_euclid(std::f64::consts::TAU))),
        "graphics.fittingCoverage" => view::coverage(&args),
        "graphics.equipmentPose" => Ok(if args["weapon"] == "pistol" {
            json!({"position":[0.25,0.94,0.15],"rotation":[0.,0.,-std::f64::consts::FRAC_PI_2]})
        } else {
            json!({"position":[-0.035,1.18,0.215],"rotation":[0.,0.,-0.96]})
        }),
        "graphics.backdrop" => view::backdrop(&args),
        "graphics.backdropShader" => Ok(json!({
            "vertex":"varying vec2 vUv; void main() { vUv = uv; gl_Position = vec4(position.xy, 0., 1.); }",
            "fragment":include_str!("depth-backdrop.glsl")
        })),
        "graphics.stage" => Ok(stage::stage()),
        "graphics.sling" => stage::sling(&args),
        "graphics.carry" => carry::carry(&args),
        "graphics.pomShaders" => Ok(json!({
            "common": include_str!("silhouette-pom.glsl"),
            "trace": include_str!("silhouette-trace.glsl"),
            "normal": "float faceDirection = gl_FrontFacing ? 1.0 : -1.0;\nvec3 normal = normalize(cdPomNormalMatrix * cdPomNormal(cdPomHit));\nvec3 nonPerturbedNormal = normal;",
            "version": "cd-finite-relief-v1"
        })),
        "graphics.surfacePom" => surface::program(&args),
        "graphics.surfaceProfile" => Ok(surface::profile(&args)),
        "graphics.pomQuality" => {
            let quality = args["quality"].as_str().unwrap_or("medium");
            let (steps, refine, texture_size) = match quality {
                "low" => (0, 0, 512),
                "medium" => (28, 5, 1024),
                "high" => (48, 6, 1024),
                "ultra" => (72, 7, 1024),
                _ => return Err(format!("Unknown graphics quality: {quality}")),
            };
            Ok(
                json!({"steps":steps, "refine":refine, "textureSize":texture_size, "enabled":steps > 0}),
            )
        }
        "graphics.weaponVisibility" => {
            let nodes = args["nodes"].as_array().ok_or("Missing weapon nodes")?;
            let loadout = &args["loadout"];
            let mut replacing = Vec::new();
            let mut result = Vec::new();
            for node in nodes {
                if node["factory"].as_bool() != Some(false) {
                    continue;
                }
                let selected = loadout["weapon1"]["mods"][node["slot"].as_str().unwrap_or("")]
                    .as_str()
                    .unwrap_or("");
                let mesh = selected.replace('-', "_");
                let visible = !mesh.is_empty() && node["name"] == mesh;
                if visible {
                    if let Some(name) = node["replaces"].as_str() {
                        replacing.push(name);
                    }
                }
                result.push(json!({"name":node["name"],"visible":visible}));
            }
            for node in nodes {
                if node["factory"].as_bool() == Some(true) {
                    result.push(json!({"name":node["name"],"visible":!replacing.contains(&node["name"].as_str().unwrap_or(""))}));
                }
            }
            Ok(json!(result))
        }
        _ => Err(format!("Unknown graphics operation: {method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_quality_has_an_explicit_geometric_fallback() {
        let low = dispatch("graphics.pomQuality", json!({"quality":"low"})).unwrap();
        assert_eq!(low["enabled"], false);
        assert_eq!(low["steps"], 0);
        assert_eq!(
            dispatch("graphics.pomQuality", json!({"quality":"ultra"})).unwrap()["steps"],
            72
        );
    }

    #[test]
    fn replaces_only_selected_factory_components() {
        let r = dispatch("graphics.weaponVisibility", json!({
            "loadout":{"weapon1":{"mods":{"muzzle":"muzzle-tishina"}}},
            "nodes":[
                {"name":"muzzle_tishina","factory":false,"slot":"muzzle","replaces":"factory_muzzle"},
                {"name":"optic_sokol","factory":false,"slot":"optic"},
                {"name":"factory_muzzle","factory":true},
                {"name":"receiver","factory":true}
            ]
        })).unwrap();
        assert_eq!(r[0]["visible"], true);
        assert_eq!(r[1]["visible"], false);
        assert_eq!(r[2]["visible"], false);
        assert_eq!(r[3]["visible"], true);
    }
}
