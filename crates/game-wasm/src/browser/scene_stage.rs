//! Fitting-room scene graph, silhouette materials and backdrop lifetime.
use super::{bridge::*, scene_assets::*};
use js_sys::{Array, Object, Promise, WeakMap};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

thread_local! {static HOOKS:WeakMap=WeakMap::new();}
fn texture(host: &JsValue, kind: &str, size: u32) -> JsValue {
    let bytes = invoke(&get(host, "graphicsTexture"), &[kind.into(), size.into()]);
    let texture = make(
        host,
        "DataTexture",
        &[bytes, size.into(), size.into(), three(host, "RGBAFormat")],
    );
    set(&texture, "magFilter", &three(host, "LinearFilter"));
    set(
        &texture,
        "minFilter",
        &three(host, "LinearMipmapLinearFilter"),
    );
    set(&texture, "generateMipmaps", &true.into());
    set(&texture, "anisotropy", &4.into());
    set(&texture, "needsUpdate", &true.into());
    texture
}
#[wasm_bindgen(js_name=sceneTurntableMaterial)]
pub fn turntable_material(host: JsValue) -> JsValue {
    let map = texture(&host, "steel-color", 1024);
    set(&map, "colorSpace", &three(&host, "SRGBColorSpace"));
    make(
        &host,
        "MeshStandardMaterial",
        &[object(&[
            ("color", 0xb5b8bau32.into()),
            ("map", map),
            ("roughness", 0.98.into()),
            ("roughnessMap", texture(&host, "steel-roughness", 1024)),
            ("metalness", 0.42.into()),
            ("bumpMap", texture(&host, "steel-relief", 1024)),
            ("bumpScale", 0.004.into()),
        ])],
    )
}
#[wasm_bindgen(js_name=sceneTuneFittingMaterial)]
pub fn tune_fitting_material(host: JsValue, material: JsValue, quality: JsValue) {
    let hook = HOOKS.with(|h| h.get(material.unchecked_ref::<Object>()));
    if present(&hook) {
        invoke(&get(&hook, "quality"), &[get(&quality, "id")]);
    }
    tune(
        host,
        material,
        quality,
        get(&hook, "compile"),
        string(&get(&hook, "key")),
    );
}
fn patch_common(shader: &JsValue, uniforms: &JsValue, program: &JsValue) {
    Object::assign(
        &get(shader, "uniforms").unchecked_into::<Object>(),
        &uniforms.clone().unchecked_into::<Object>(),
    );
    let vertex = format!(
        "{}{}",
        include_str!("../../shaders/podium-prefix.glsl"),
        string(&get(shader, "vertexShader"))
    )
    .replacen(
        "#include <begin_vertex>",
        include_str!("../../shaders/podium-begin.glsl"),
        1,
    );
    let fragment = string(&get(shader, "fragmentShader"))
        .replacen(
            "#include <common>",
            &format!("#include <common>\n{}", string(&get(program, "common"))),
            1,
        )
        .replacen(
            "void main() {",
            &format!("void main() {{\n{}", string(&get(program, "trace"))),
            1,
        );
    set(shader, "vertexShader", &vertex.into());
    set(shader, "fragmentShader", &fragment.into());
}
fn camera_update(
    mesh: &JsValue,
    uniforms: &JsValue,
    camera: &JsValue,
    inverse_world: &JsValue,
    inverse_view: &JsValue,
) {
    call(inverse_world, "copy", &[get(mesh, "matrixWorld")]);
    call(inverse_world, "invert", &[]);
    let local = uv(uniforms, "cdPomCameraLocal");
    call(
        &local,
        "setFromMatrixPosition",
        &[get(camera, "matrixWorld")],
    );
    call(&local, "applyMatrix4", &[inverse_world.clone()]);
    let ray = uv(uniforms, "cdPomRayLocal");
    call(camera, "getWorldDirection", &[ray.clone()]);
    call(&ray, "transformDirection", &[inverse_world.clone()]);
    put(
        uniforms,
        "cdPomOrthographic",
        boolean(&get(camera, "isOrthographicCamera")).into(),
    );
    call(
        &uv(uniforms, "cdPomModelWorld"),
        "copy",
        &[get(mesh, "matrixWorld")],
    );
    let model_view = uv(uniforms, "cdPomModelView");
    call(
        &model_view,
        "multiplyMatrices",
        &[get(camera, "matrixWorldInverse"), get(mesh, "matrixWorld")],
    );
    call(
        &uv(uniforms, "cdPomProjection"),
        "copy",
        &[get(camera, "projectionMatrix")],
    );
    call(
        &uv(uniforms, "cdPomNormalMatrix"),
        "getNormalMatrix",
        &[model_view.clone()],
    );
    call(inverse_view, "copy", &[model_view]);
    call(inverse_view, "invert", &[]);
    call(
        &uv(uniforms, "cdPomViewToLocal"),
        "setFromMatrix4",
        &[inverse_view.clone()],
    );
}
#[wasm_bindgen(js_name=sceneSilhouetteDeck)]
pub fn silhouette_deck(host: JsValue, source: JsValue, radius: f64, height: f64) -> JsValue {
    let program = compute(&host, "graphics.pomShaders", Object::new().into());
    let material = call(&source, "clone", &[]);
    set(&material, "name", &"Rust_Silhouette_Relief_Steel".into());
    set(&material, "bumpMap", &JsValue::NULL);
    set(&material, "shadowSide", &three(&host, "FrontSide"));
    let mesh = make(
        &host,
        "Mesh",
        &[
            make(
                &host,
                "BoxGeometry",
                &[(radius * 2.).into(), height.into(), (radius * 2.).into()],
            ),
            material.clone(),
        ],
    );
    let fallback = make(
        &host,
        "Mesh",
        &[
            make(
                &host,
                "CylinderGeometry",
                &[radius.into(), radius.into(), height.into(), 96.into()],
            ),
            source.clone(),
        ],
    );
    for (object, name) in [
        (&mesh, "Silhouette_POM_Finite_Volume"),
        (&fallback, "Low_Quality_Steel_Deck"),
    ] {
        set(object, "name", &name.into());
        set(object, "castShadow", &true.into());
        set(object, "receiveShadow", &true.into());
    }
    let group = make(&host, "Group", &[]);
    call(&group, "add", &[mesh.clone(), fallback.clone()]);
    let uniforms = Object::new();
    for (key, value) in [
        ("cdPomRelief", get(&source, "bumpMap")),
        (
            "cdPomSize",
            make(&host, "Vector2", &[radius.into(), (height / 2.).into()]),
        ),
        ("cdPomCameraLocal", make(&host, "Vector3", &[])),
        ("cdPomRayLocal", make(&host, "Vector3", &[])),
        ("cdPomOrthographic", false.into()),
        ("cdPomSteps", 28.into()),
        ("cdPomRefine", 5.into()),
        ("cdPomShadowSteps", 10.into()),
        ("cdPomTexel", (1. / 1024.).into()),
        ("cdPomModelView", make(&host, "Matrix4", &[])),
        ("cdPomModelWorld", make(&host, "Matrix4", &[])),
        ("cdPomProjection", make(&host, "Matrix4", &[])),
        ("cdPomNormalMatrix", make(&host, "Matrix3", &[])),
        ("cdPomViewToLocal", make(&host, "Matrix3", &[])),
    ] {
        set(&uniforms, key, &uniform(value));
    }
    let uniforms: JsValue = uniforms.into();
    let inverse_world = make(&host, "Matrix4", &[]);
    let inverse_view = make(&host, "Matrix4", &[]);
    let (m, u, iw, iv) = (
        mesh.clone(),
        uniforms.clone(),
        inverse_world.clone(),
        inverse_view.clone(),
    );
    set(
        &mesh,
        "onBeforeRender",
        &Closure::wrap(Box::new(move |_: JsValue, _: JsValue, camera: JsValue| {
            camera_update(&m, &u, &camera, &iw, &iv)
        }) as Box<dyn FnMut(JsValue, JsValue, JsValue)>)
        .into_js_value(),
    );
    let (m, u) = (mesh.clone(), uniforms.clone());
    set(
        &mesh,
        "onBeforeShadow",
        &Closure::wrap(
            Box::new(move |_: JsValue, _: JsValue, _: JsValue, camera: JsValue| {
                camera_update(&m, &u, &camera, &inverse_world, &inverse_view)
            }) as Box<dyn FnMut(JsValue, JsValue, JsValue, JsValue)>,
        )
        .into_js_value(),
    );
    let (h, u, p) = (host.clone(), uniforms.clone(), program.clone());
    let compile=Closure::wrap(Box::new(move|shader:JsValue|{
        patch_common(&shader,&u,&p);
        let chunks=three(&h,"ShaderChunk");
        let mut lighting=string(&get(&chunks,"lights_fragment_begin")).replace("vViewPosition","cdPomViewPosition")
            .replace("vDirectionalShadowCoord[ i ]","(vDirectionalShadowCoord[ i ] + directionalShadowMatrix[ i ] * vec4(cdPomWorldDelta, 0.0))")
            .replace("vPointShadowCoord[ i ]","(vPointShadowCoord[ i ] + pointShadowMatrix[ i ] * vec4(cdPomWorldDelta, 0.0))")
            .replace("vSpotLightCoord[ i ]","(vSpotLightCoord[ i ] + spotLightMatrix[ i ] * vec4(cdPomWorldDelta, 0.0))");
        lighting=lighting.replace("RE_Direct( directLight,","directLight.color *= cdPomLightVisibility(directLight.direction);\n\t\tRE_Direct( directLight,");
        let fragment=string(&get(&shader,"fragmentShader"))
            .replacen("#include <map_fragment>",&string(&get(&chunks,"map_fragment")).replace("vMapUv","cdPomUv"),1)
            .replacen("#include <roughnessmap_fragment>",&string(&get(&chunks,"roughnessmap_fragment")).replace("vRoughnessMapUv","cdPomUv"),1)
            .replacen("#include <normal_fragment_begin>",&string(&get(&p,"normal")),1)
            .replacen("#include <lights_fragment_begin>",&lighting,1)
            .replacen("#include <common>",&format!("#include <common>\n{}",include_str!("../../shaders/podium-shadow-matrices.glsl")),1);
        set(&shader,"fragmentShader",&fragment.into());
    })as Box<dyn FnMut(JsValue)>).into_js_value();
    let depth = make(
        &host,
        "MeshDepthMaterial",
        &[object(&[(
            "depthPacking",
            three(&host, "RGBADepthPacking"),
        )])],
    );
    let (u, p) = (uniforms.clone(), program.clone());
    set(
        &depth,
        "onBeforeCompile",
        &Closure::wrap(Box::new(move |shader: JsValue| {
            patch_common(&shader, &u, &p);
            let fragment = replace_regex(
                &string(&get(&shader, "fragmentShader")),
                "float fragCoordZ = [^;]+;",
                "g",
                "float fragCoordZ = cdPomDepth;",
            );
            set(&shader, "fragmentShader", &fragment.into());
        }) as Box<dyn FnMut(JsValue)>)
        .into_js_value(),
    );
    let distance = make(&host, "MeshDistanceMaterial", &[]);
    let (u, p) = (uniforms.clone(), program.clone());
    set(
        &distance,
        "onBeforeCompile",
        &Closure::wrap(Box::new(move |shader: JsValue| {
            patch_common(&shader, &u, &p);
            let fragment = string(&get(&shader, "fragmentShader")).replacen(
                "length( vWorldPosition - referencePosition )",
                "length( cdPomWorldHit - referencePosition )",
                1,
            );
            set(&shader, "fragmentShader", &fragment.into());
        }) as Box<dyn FnMut(JsValue)>)
        .into_js_value(),
    );
    let version = string(&get(&program, "version"));
    for (mat, key) in [
        (&material, version.clone()),
        (&depth, format!("{version}-depth")),
        (&distance, format!("{version}-distance")),
    ] {
        set(mat,"customProgramCacheKey",&Closure::wrap(Box::new(move||JsValue::from_str(&key))as Box<dyn FnMut()->JsValue>).into_js_value());
    }
    set(&mesh, "customDepthMaterial", &depth);
    set(&mesh, "customDistanceMaterial", &distance);
    let cleanup_mesh = mesh.clone();
    let default_render = get(
        &get(&three(&host, "Object3D"), "prototype"),
        "onBeforeRender",
    );
    let default_shadow = get(
        &get(&three(&host, "Object3D"), "prototype"),
        "onBeforeShadow",
    );
    let cleanup = Closure::wrap(Box::new(move |event: JsValue| {
        let owner = get(&event, "target");
        let listener = get(&owner, "__cdReliefDispose");
        call(&owner, "removeEventListener", &["dispose".into(), listener]);
        set(&owner, "__cdReliefDispose", &JsValue::UNDEFINED);
        HOOKS.with(|hooks| {
            hooks.delete(owner.unchecked_ref::<Object>());
        });
        set(&cleanup_mesh, "onBeforeRender", &default_render);
        set(&cleanup_mesh, "onBeforeShadow", &default_shadow);
        call(&depth, "dispose", &[]);
        call(&distance, "dispose", &[]);
    }) as Box<dyn FnMut(JsValue)>)
    .into_js_value();
    set(&material, "__cdReliefDispose", &cleanup);
    call(&material, "addEventListener", &["dispose".into(), cleanup]);
    let (h, g) = (host.clone(), group.clone());
    let set_quality = Closure::wrap(Box::new(move |quality: JsValue| {
        let selected = compute(
            &h,
            "graphics.pomQuality",
            object(&[("quality", quality.clone())]),
        );
        let enabled = boolean(&get(&selected, "enabled"));
        set(&mesh, "visible", &enabled.into());
        set(&fallback, "visible", &(!enabled).into());
        put(&uniforms, "cdPomSteps", get(&selected, "steps"));
        put(&uniforms, "cdPomRefine", get(&selected, "refine"));
        put(&uniforms, "cdPomShadowSteps", get(&selected, "shadowSteps"));
        set(&get(&g, "userData"), "reliefQuality", &quality);
        set(&get(&g, "userData"), "reliefEnabled", &enabled.into());
    }) as Box<dyn FnMut(JsValue)>)
    .into_js_value();
    HOOKS.with(|hooks| {
        hooks.set(
            material.unchecked_ref::<Object>(),
            &object(&[
                ("compile", compile.clone()),
                ("key", version.into()),
                ("quality", set_quality.clone()),
            ]),
        );
    });
    set(&material, "onBeforeCompile", &compile);
    invoke(&set_quality, &["medium".into()]);
    object(&[("group", group), ("setQuality", set_quality)])
}

fn geometry(host: &JsValue, spec: &JsValue) -> JsValue {
    let args = get(spec, "args");
    let args = if present(&args) {
        values(&args)
    } else {
        Vec::new()
    };
    let kind = string(&get(spec, "kind"));
    match kind.as_str() {
        "plane" => make(host, "PlaneGeometry", &args),
        "box" => make(host, "BoxGeometry", &args),
        "cylinder" => make(host, "CylinderGeometry", &args),
        "torus" => make(host, "TorusGeometry", &args),
        "lathe" => {
            let points = values(&get(spec, "profile"))
                .iter()
                .map(|p| make(host, "Vector2", &values(p)))
                .collect::<Vec<_>>();
            make(
                host,
                "LatheGeometry",
                &[array(&points).into(), get(spec, "segments")],
            )
        }
        "tube" => {
            let points = values(&get(spec, "points"))
                .iter()
                .map(|p| make(host, "Vector3", &values(p)))
                .collect::<Vec<_>>();
            let path = make(host, "CatmullRomCurve3", &[array(&points).into()]);
            make(
                host,
                "TubeGeometry",
                &[
                    path,
                    get(spec, "segments"),
                    get(spec, "radius"),
                    get(spec, "radialSegments"),
                    false.into(),
                ],
            )
        }
        _ => wasm_bindgen::throw_str(&format!("Unknown Rust stage primitive: {kind}")),
    }
}

/// One authored accessory surface: positions, normals, indices and a plain material.
fn accessory(host: &JsValue, part: &JsValue) -> JsValue {
    let geometry = make(host, "BufferGeometry", &[]);
    for (attribute, field) in [("position", "positions"), ("normal", "normals")] {
        let data = make(
            host,
            "Float32BufferAttribute",
            &[get(part, field), 3.into()],
        );
        call(&geometry, "setAttribute", &[attribute.into(), data]);
    }
    call(&geometry, "setIndex", &[get(part, "indices")]);
    let color = make(host, "Color", &[]);
    let rgb = get(part, "color");
    call(&color, "setRGB", &[at(&rgb, 0), at(&rgb, 1), at(&rgb, 2)]);
    let material = make(
        host,
        "MeshStandardMaterial",
        &[object(&[
            ("color", color),
            ("roughness", get(part, "roughness")),
            ("metalness", get(part, "metalness")),
            ("side", three(host, "DoubleSide")),
        ])],
    );
    let mesh = make(host, "Mesh", &[geometry, material]);
    set(&mesh, "name", &get(part, "name"));
    set(&mesh, "castShadow", &true.into());
    set(&mesh, "receiveShadow", &true.into());
    mesh
}
/// Preserve the user's evaluated Blender accessory surfaces and authored weapon matrix.
#[wasm_bindgen(js_name = sceneAuthoredWeaponFit)]
pub fn authored_weapon_fit(host: JsValue, mount: JsValue, layout: JsValue) -> JsValue {
    let matrix = make(&host, "Matrix4", &[]);
    call(&matrix, "fromArray", &[get(&layout, "weaponMatrix")]);
    call(
        &matrix,
        "decompose",
        &[
            get(&mount, "position"),
            get(&mount, "quaternion"),
            get(&mount, "scale"),
        ],
    );
    let group = make(&host, "Group", &[]);
    set(&group, "name", &"Authored_Sling_And_Clips".into());
    for part in values(&get(&layout, "accessories")) {
        call(&group, "add", &[accessory(&host, &part)]);
    }
    group
}
#[wasm_bindgen(js_name=sceneMakeFittingStage)]
pub fn make_stage(host: JsValue) -> JsValue {
    let spec = compute(&host, "graphics.stage", Object::new().into());
    let fixed = make(&host, "Group", &[]);
    let rotating = make(&host, "Group", &[]);
    let feet = make(&host, "Group", &[]);
    for (group, name) in [
        (&fixed, "Fitting_Bay_Fixed_Floor_And_Bearing"),
        (&rotating, "Fitting_Turntable_Persona_And_Sling"),
        (&feet, "Foot_Contact_Shadows"),
    ] {
        set(group, "name", &name.into());
    }
    let soft = make(
        &host,
        "DataTexture",
        &[
            invoke(
                &get(&host, "graphicsTexture"),
                &["contact".into(), 128.into()],
            ),
            128.into(),
            128.into(),
            three(&host, "RGBAFormat"),
        ],
    );
    for key in ["magFilter", "minFilter"] {
        set(&soft, key, &three(&host, "LinearFilter"));
    }
    set(&soft, "needsUpdate", &true.into());
    let deck = turntable_material(host.clone());
    let steel = call(&deck, "clone", &[]);
    call(&get(&steel, "color"), "setHex", &[0x92989bu32.into()]);
    set(&steel, "metalness", &0.5.into());
    let edge = call(&deck, "clone", &[]);
    call(&get(&edge, "color"), "setHex", &[0xd2d5d5u32.into()]);
    set(&edge, "roughness", &0.69.into());
    set(&edge, "metalness", &0.75.into());
    let materials = object(&[
        ("deck", deck.clone()),
        ("steel", steel),
        ("edge", edge),
        (
            "rubber",
            make(
                &host,
                "MeshStandardMaterial",
                &[object(&[
                    ("color", 0x111111u32.into()),
                    ("roughness", 0.97.into()),
                    ("metalness", 0.into()),
                ])],
            ),
        ),
        (
            "paint",
            make(
                &host,
                "MeshStandardMaterial",
                &[object(&[
                    ("color", 0xa5aaa7u32.into()),
                    ("roughness", 0.8.into()),
                    ("metalness", 0.08.into()),
                ])],
            ),
        ),
        (
            "floor",
            make(
                &host,
                "ShadowMaterial",
                &[object(&[
                    ("color", 0x101010u32.into()),
                    ("opacity", 0.32.into()),
                    ("depthWrite", false.into()),
                ])],
            ),
        ),
    ]);
    let mut setters = vec![];
    for (key, group) in [("fixed", &fixed), ("rotating", &rotating), ("feet", &feet)] {
        for part in values(&get(&spec, key)) {
            let d = get(&part, "geometry");
            let result = if string(&get(&d, "kind")) == "silhouetteDeck" {
                let relief = silhouette_deck(
                    host.clone(),
                    deck.clone(),
                    num(&get(&d, "radius")),
                    num(&get(&d, "height")),
                );
                setters.push(get(&relief, "setQuality"));
                get(&relief, "group")
            } else {
                let name = string(&get(&part, "material"));
                let material = if name == "contact" {
                    make(
                        &host,
                        "MeshBasicMaterial",
                        &[object(&[
                            ("color", 0x030405u32.into()),
                            ("map", soft.clone()),
                            ("transparent", true.into()),
                            ("opacity", get(&part, "opacity")),
                            ("depthWrite", false.into()),
                            ("toneMapped", false.into()),
                        ])],
                    )
                } else {
                    get(&materials, &name)
                };
                let mesh = make(&host, "Mesh", &[geometry(&host, &d), material]);
                set(&mesh, "castShadow", &get(&part, "castShadow"));
                set(&mesh, "receiveShadow", &get(&part, "receiveShadow"));
                mesh
            };
            call(
                &get(&result, "position"),
                "fromArray",
                &[get(&part, "position")],
            );
            call(
                &get(&result, "rotation"),
                "set",
                &values(&get(&part, "rotation")),
            );
            if present(&get(&part, "name")) {
                set(&result, "name", &get(&part, "name"));
            }
            call(group, "add", &[result]);
        }
    }
    call(&rotating, "add", &[feet.clone()]);
    let quality = Closure::wrap(Box::new(move |quality: JsValue| {
        for setter in &setters {
            invoke(setter, &[quality.clone()]);
        }
    }) as Box<dyn FnMut(JsValue)>)
    .into_js_value();
    object(&[
        ("fixed", fixed),
        ("rotating", rotating),
        ("feet", feet),
        ("setQuality", quality),
    ])
}
/// A carabiner sling clip: a bent steel loop and its gate, in the clip's own plane.
fn sling_clip(host: &JsValue, name: &str, clip_path: &JsValue) -> JsValue {
    let clip = make(host, "Group", &[]);
    set(&clip, "name", &name.into());
    let metal = make(
        host,
        "MeshStandardMaterial",
        &[object(&[
            ("color", 0x666b6cu32.into()),
            ("metalness", 0.8.into()),
            ("roughness", 0.48.into()),
        ])],
    );
    let points = values(clip_path)
        .iter()
        .map(|p| make(host, "Vector3", &values(p)))
        .collect::<Vec<_>>();
    let path = make(host, "CatmullRomCurve3", &[array(&points).into()]);
    let frame = make(
        host,
        "Mesh",
        &[
            make(
                host,
                "TubeGeometry",
                &[path, 24.into(), 0.0024.into(), 6.into(), false.into()],
            ),
            metal.clone(),
        ],
    );
    set(&frame, "name", &format!("{name}_Frame").into());
    let gate = make(
        host,
        "Mesh",
        &[
            make(
                host,
                "CylinderGeometry",
                &[0.0019.into(), 0.0019.into(), 0.031.into(), 8.into()],
            ),
            metal,
        ],
    );
    set(&gate, "name", &format!("{name}_Gate").into());
    call(
        &get(&gate, "position"),
        "set",
        &[0.009.into(), (-0.001).into(), 0.into()],
    );
    call(&clip, "add", &[frame, gate]);
    clip
}
#[wasm_bindgen(js_name=sceneMakeSling)]
pub fn make_sling(host: JsValue, mount: JsValue, pistol: bool) -> JsValue {
    call(&mount, "updateMatrix", &[]);
    let spec = compute(
        &host,
        "graphics.sling",
        object(&[
            ("matrix", get(&get(&mount, "matrix"), "elements")),
            ("pistol", pistol.into()),
        ]),
    );
    let geometry = make(&host, "BufferGeometry", &[]);
    for (key, size) in [("position", 3), ("uv", 2)] {
        call(
            &geometry,
            "setAttribute",
            &[
                key.into(),
                make(
                    &host,
                    "Float32BufferAttribute",
                    &[get(&spec, key), size.into()],
                ),
            ],
        );
    }
    call(&geometry, "setIndex", &[get(&spec, "index")]);
    call(&geometry, "computeVertexNormals", &[]);
    let sling = make(
        &host,
        "Mesh",
        &[
            geometry,
            make(
                &host,
                "MeshStandardMaterial",
                &[object(&[
                    ("color", 0x20251fu32.into()),
                    ("roughness", 0.94.into()),
                    ("metalness", 0.into()),
                    ("side", three(&host, "DoubleSide")),
                ])],
            ),
        ],
    );
    set(&sling, "name", &"Two_Point_Woven_Sling".into());
    set(&sling, "castShadow", &true.into());
    set(&sling, "receiveShadow", &true.into());
    for descriptor in values(&get(&spec, "clips")) {
        let name = string(&get(&descriptor, "name"));
        let clip = sling_clip(&host, &name, &get(&spec, "clipPath"));
        call(
            &get(&clip, "position"),
            "fromArray",
            &[get(&descriptor, "position")],
        );
        set(
            &get(&clip, "rotation"),
            "z",
            &get(&get(&mount, "rotation"), "z"),
        );
        call(&sling, "add", &[clip]);
    }
    sling
}
/// Put `target` where a fitted column-major matrix says.
fn place(host: &JsValue, target: &JsValue, elements: &JsValue) {
    let matrix = make(host, "Matrix4", &[]);
    call(&matrix, "fromArray", &[elements.clone()]);
    call(
        &matrix,
        "decompose",
        &[
            get(target, "position"),
            get(target, "quaternion"),
            get(target, "scale"),
        ],
    );
}
/// The gun as it is carried by this Persona's fitting pose: the mount moves to its fitted place and the returned group
/// holds the straps lying on the clothes, the sling clips and the holster. Null when the pose has no fit for the gun.
#[wasm_bindgen(js_name=sceneMakeCarry)]
pub fn make_carry(host: JsValue, mount: JsValue, sex: String, weapon: String) -> JsValue {
    let spec = compute(
        &host,
        "graphics.carry",
        object(&[("sex", sex.into()), ("weapon", weapon.into())]),
    );
    if !present(&spec) {
        return JsValue::NULL;
    }
    place(&host, &mount, &get(&spec, "matrix"));
    let group = make(&host, "Group", &[]);
    set(&group, "name", &"Fitted_Carry".into());
    let cloth = make(
        &host,
        "MeshStandardMaterial",
        &[object(&[
            ("color", 0x20251fu32.into()),
            ("roughness", 0.94.into()),
            ("metalness", 0.into()),
            ("side", three(&host, "DoubleSide")),
        ])],
    );
    for (i, strap) in values(&get(&spec, "straps")).iter().enumerate() {
        let geometry = make(&host, "BufferGeometry", &[]);
        for (key, size) in [("position", 3), ("uv", 2)] {
            let data = make(
                &host,
                "Float32BufferAttribute",
                &[get(strap, key), size.into()],
            );
            call(&geometry, "setAttribute", &[key.into(), data]);
        }
        call(&geometry, "setIndex", &[get(strap, "index")]);
        call(&geometry, "computeVertexNormals", &[]);
        let mesh = make(&host, "Mesh", &[geometry, cloth.clone()]);
        set(&mesh, "name", &format!("Carry_Strap_{i}").into());
        set(&mesh, "castShadow", &true.into());
        set(&mesh, "receiveShadow", &true.into());
        call(&group, "add", &[mesh]);
    }
    for descriptor in values(&get(&spec, "clips")) {
        let name = string(&get(&descriptor, "name"));
        let clip = sling_clip(&host, &name, &get(&spec, "clipPath"));
        place(&host, &clip, &get(&descriptor, "matrix"));
        call(&group, "add", &[clip]);
    }
    if let Some(url) = get(&spec, "holster").as_string() {
        let holster = make(&host, "Group", &[]);
        set(&holster, "name", &"Drop_Leg_Holster".into());
        place(&host, &holster, &get(&spec, "matrix"));
        call(&group, "add", &[holster.clone()]);
        // Each carry loads its own copy: a carry that goes away frees its textures' images with it.
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(gltf) = await_value(invoke(&get(&host, "loadModel"), &[url.into()])).await else {
                return;
            };
            let model = get(&gltf, "scene");
            for node in nodes(&model) {
                set(&node, "castShadow", &true.into());
                set(&node, "receiveShadow", &true.into());
            }
            call(&holster, "add", &[model]);
        });
    }
    group
}
#[wasm_bindgen(js_name=scenePrepareVaryag)]
pub fn prepare(gun: JsValue) {
    prepare_varyag(&gun);
}
#[wasm_bindgen(js_name=sceneFitVaryag)]
pub fn fit_varyag(host: JsValue, gun: JsValue, loadout: JsValue) {
    let descriptors = Array::new();
    for node in nodes(&gun) {
        let data = get(&node, "userData");
        if get(&data, "factory").as_bool().is_some() {
            descriptors.push(&object(&[
                ("name", get(&node, "name")),
                ("factory", get(&data, "factory")),
                ("slot", get(&data, "slot")),
                ("replaces", get(&data, "replaces")),
            ]));
        }
    }
    let visibility = compute(
        &host,
        "graphics.weaponVisibility",
        object(&[("nodes", descriptors.into()), ("loadout", loadout)]),
    );
    for state in values(&visibility) {
        let node = call(&gun, "getObjectByName", &[get(&state, "name")]);
        if present(&node) {
            set(&node, "visible", &get(&state, "visible"));
        }
    }
}

struct BackdropState {
    host: JsValue,
    renderer: JsValue,
    scene: JsValue,
    camera: JsValue,
    material: JsValue,
    geometry: JsValue,
    uniforms: JsValue,
    canvas: JsValue,
    ratio: f64,
    zoom: f64,
    width: f64,
    height: f64,
    last: String,
    clock: super::live::Clock,
    disposed: bool,
}
#[wasm_bindgen]
pub struct RustDepthBackdrop {
    state: Rc<RefCell<BackdropState>>,
    ready: Promise,
}
#[wasm_bindgen]
impl RustDepthBackdrop {
    #[wasm_bindgen(constructor)]
    pub fn new(
        host: JsValue,
        canvas: JsValue,
        color: String,
        depth: String,
        ratio: f64,
        zoom: f64,
    ) -> Self {
        let renderer = make(
            &host,
            "WebGLRenderer",
            &[object(&[
                ("canvas", canvas.clone()),
                ("alpha", false.into()),
                ("antialias", false.into()),
                ("powerPreference", "low-power".into()),
            ])],
        );
        set(
            &renderer,
            "outputColorSpace",
            &three(&host, "LinearSRGBColorSpace"),
        );
        let scene = make(&host, "Scene", &[]);
        let camera = make(
            &host,
            "OrthographicCamera",
            &[
                (-1).into(),
                1.into(),
                1.into(),
                (-1).into(),
                0.into(),
                1.into(),
            ],
        );
        let uniforms = object(&[
            ("uColor", uniform(JsValue::NULL)),
            ("uDepth", uniform(JsValue::NULL)),
            (
                "uCover",
                uniform(make(&host, "Vector2", &[1.into(), 1.into()])),
            ),
            ("uShift", uniform(make(&host, "Vector2", &[]))),
        ]);
        // The room's life is found by its picture; Sedoy's portrait also talks (briefing_portrait.rs).
        let portrait = color.ends_with(super::briefing_portrait::PICTURE);
        let life = if portrait {
            &super::live::SEDOY
        } else {
            super::base_region::REGIONS
                .iter()
                .find(|r| {
                    let lite = r.fitting.replace(".png", ".lite.webp");
                    color.ends_with(r.fitting) || color.ends_with(&lite) || color.ends_with(r.fitting_sharp)
                })
                .map_or(&super::live::STILL, |r| r.fitting_life)
        };
        let vec4 = |v: [f64; 4]| make(&host, "Vector4", &v.map(JsValue::from));
        for (key, value) in super::live::uniforms(life, ratio, &vec4) {
            set(&uniforms, key, &value);
        }
        let face = portrait.then_some(&super::briefing_portrait::FACE);
        let rects = face.map_or([[0.0; 4]; 3], |f| [f.mouth, f.eyes, f.head]);
        set(&uniforms, "uFace", &uniform(JsValue::NULL));
        set(&uniforms, "uFaceMix", &uniform(vec4([0.0; 4])));
        for (key, rect) in ["uMouthRect", "uEyesRect", "uHead"].into_iter().zip(rects) {
            set(&uniforms, key, &uniform(vec4(rect)));
        }
        let shapes = face.map(|f| color.replace(super::briefing_portrait::PICTURE, f.shapes));
        let shader = compute(&host, "graphics.backdropShader", Object::new().into());
        let material = make(
            &host,
            "ShaderMaterial",
            &[object(&[
                ("uniforms", uniforms.clone()),
                ("vertexShader", get(&shader, "vertex")),
                ("fragmentShader", get(&shader, "fragment")),
                ("depthTest", false.into()),
                ("depthWrite", false.into()),
            ])],
        );
        let geometry = make(&host, "PlaneGeometry", &[2.into(), 2.into()]);
        let quad = make(&host, "Mesh", &[geometry.clone(), material.clone()]);
        set(&quad, "frustumCulled", &false.into());
        call(&scene, "add", &[quad]);
        let state = Rc::new(RefCell::new(BackdropState {
            host: host.clone(),
            renderer,
            scene,
            camera,
            material,
            geometry,
            uniforms,
            canvas,
            ratio,
            zoom,
            width: 0.,
            height: 0.,
            last: String::new(),
            clock: super::live::Clock::default(),
            disposed: false,
        }));
        let weak = Rc::downgrade(&state);
        let ready = future_to_promise(async move {
            let loader = make(&host, "TextureLoader", &[]);
            let tasks = Array::new();
            for url in [Some(color), Some(depth), shapes].into_iter().flatten() {
                let host = host.clone();
                let pending = call(&loader, "loadAsync", &[url.into()]);
                tasks.push(&future_to_promise(async move {
                    let t = await_value(pending).await?;
                    set(&t, "colorSpace", &three(&host, "NoColorSpace"));
                    for key in ["wrapS", "wrapT"] {
                        set(&t, key, &three(&host, "ClampToEdgeWrapping"));
                    }
                    set(&t, "generateMipmaps", &false.into());
                    set(&t, "minFilter", &three(&host, "LinearFilter"));
                    Ok(t)
                }));
            }
            // allSettled retains successful textures if its partner fails, so neither leaks.
            let results = await_value(call(
                &get(&global(), "Promise"),
                "allSettled",
                &[tasks.into()],
            ))
            .await?;
            let mut loaded = vec![];
            let mut error = None;
            for result in values(&results) {
                if string(&get(&result, "status")) == "fulfilled" {
                    loaded.push(get(&result, "value"));
                } else {
                    error = Some(get(&result, "reason"));
                }
            }
            let target = weak.upgrade();
            let alive = target
                .as_ref()
                .is_some_and(|state| !state.borrow().disposed);
            if error.is_some() || !alive {
                for t in loaded {
                    call(&t, "dispose", &[]);
                }
                return error.map_or(Ok(JsValue::UNDEFINED), Err);
            }
            let target = target.unwrap();
            {
                let mut s = target.borrow_mut();
                put(&s.uniforms, "uColor", loaded[0].clone());
                put(&s.uniforms, "uDepth", loaded[1].clone());
                if let Some(shapes) = loaded.get(2) {
                    put(&s.uniforms, "uFace", shapes.clone());
                }
                s.last.clear();
            }
            draw_backdrop(&target, 0., 0.);
            Ok(JsValue::UNDEFINED)
        });
        Self { state, ready }
    }
    #[wasm_bindgen(getter)]
    pub fn ready(&self) -> Promise {
        self.ready.clone()
    }
    pub fn draw(&self, x: f64, y: f64) {
        draw_backdrop(&self.state, x, y);
    }
    /// A talking portrait's face now: how far each mouth shape shows (slightly open, open,
    /// rounded) and how shut the eyes are, 0–1.
    pub fn face(&self, open: f64, wide: f64, round: f64, shut: f64) {
        let s = self.state.borrow();
        call(
            &uv(&s.uniforms, "uFaceMix"),
            "set",
            &[open.into(), wide.into(), round.into(), shut.into()],
        );
    }
    pub fn dispose(&self) {
        dispose_backdrop(&self.state);
    }
}
impl Drop for RustDepthBackdrop {
    fn drop(&mut self) {
        dispose_backdrop(&self.state);
    }
}
fn draw_backdrop(state: &Rc<RefCell<BackdropState>>, x: f64, y: f64) {
    let mut s = state.borrow_mut();
    if s.disposed {
        return;
    }
    let w = num(&get(&s.canvas, "clientWidth"));
    let h = num(&get(&s.canvas, "clientHeight"));
    if w == 0. || h == 0. || !present(&uv(&s.uniforms, "uDepth")) {
        return;
    }
    let dpr = num(&get(&global(), "devicePixelRatio"));
    let frame = compute(
        &s.host,
        "graphics.backdrop",
        object(&[
            ("width", w.into()),
            ("height", h.into()),
            ("ratio", s.ratio.into()),
            ("zoom", s.zoom.into()),
            ("x", x.into()),
            ("y", y.into()),
            ("dpr", if dpr == 0. { 1. } else { dpr }.into()),
        ]),
    );
    let resized = w != s.width || h != s.height;
    if resized {
        s.width = w;
        s.height = h;
        s.last.clear();
    }
    let (alive, time) = s.clock.tick();
    let mut key = format!("{x:.2} {y:.2}");
    if alive > 0. {
        key.push_str(&format!(" {alive} {time}"));
    }
    if key == s.last {
        return;
    }
    s.last = key;
    let (renderer, scene, camera, uniforms) = (
        s.renderer.clone(),
        s.scene.clone(),
        s.camera.clone(),
        s.uniforms.clone(),
    );
    drop(s);
    if resized {
        call(&renderer, "setPixelRatio", &[get(&frame, "dpr")]);
        call(&renderer, "setSize", &[w.into(), h.into(), false.into()]);
    }
    call(
        &uv(&uniforms, "uCover"),
        "fromArray",
        &[get(&frame, "cover")],
    );
    call(
        &uv(&uniforms, "uShift"),
        "fromArray",
        &[get(&frame, "shift")],
    );
    put(&uniforms, "uLive", alive.into());
    put(&uniforms, "uTime", time.into());
    call(&renderer, "render", &[scene, camera]);
}
fn dispose_backdrop(state: &Rc<RefCell<BackdropState>>) {
    let mut s = state.borrow_mut();
    if s.disposed {
        return;
    }
    s.disposed = true;
    let resources = [
        uv(&s.uniforms, "uColor"),
        uv(&s.uniforms, "uDepth"),
        uv(&s.uniforms, "uFace"),
        s.material.clone(),
        s.geometry.clone(),
        s.renderer.clone(),
    ];
    drop(s);
    for resource in resources {
        if present(&resource) {
            call(&resource, "dispose", &[]);
        }
    }
}
