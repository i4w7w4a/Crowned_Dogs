//! Scene assets and material policy. Three is an external scene-graph API only.
use super::bridge::*;
use js_sys::{Array, Object, Promise, Uint8Array, Uint32Array};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};
use wasm_bindgen::{UnwrapThrowExt, prelude::*};
use wasm_bindgen_futures::{JsFuture, future_to_promise};

pub(super) fn make(host: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    construct(&get(&get(host, "THREE"), name), args)
}
pub(super) fn three(host: &JsValue, name: &str) -> JsValue {
    get(&get(host, "THREE"), name)
}
pub(super) fn nodes(root: &JsValue) -> Vec<JsValue> {
    let mut out = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(node) = pending.pop() {
        pending.extend(values(&get(&node, "children")).into_iter().rev());
        out.push(node);
    }
    out
}
pub(super) fn compute(host: &JsValue, op: &str, input: JsValue) -> JsValue {
    invoke(&get(host, "callRust"), &[op.into(), input])
}
pub(super) fn uniform(value: JsValue) -> JsValue {
    object(&[("value", value)])
}
pub(super) fn uv(uniforms: &JsValue, key: &str) -> JsValue {
    get(&get(uniforms, key), "value")
}
pub(super) fn put(uniforms: &JsValue, key: &str, value: JsValue) {
    set(&get(uniforms, key), "value", &value);
}
pub(super) fn replace_regex(source: &str, pattern: &str, flags: &str, replacement: &str) -> String {
    let regex = js_sys::RegExp::new(pattern, flags);
    string(
        &apply(
            &get(&get(&get(&global(), "String"), "prototype"), "replace"),
            &source.into(),
            &[regex.into(), replacement.into()],
        )
        .unwrap_throw(),
    )
}
pub(super) async fn await_value(value: JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(Promise::resolve(&value)).await
}

#[wasm_bindgen(js_name = sceneLoadModel)]
pub fn load_model(host: JsValue, url: String) -> Promise {
    future_to_promise(async move {
        // The phone qualities take the model's copy with GPU-compressed maps when the build
        // ships one; a copy that fails to load or transcode falls back to the plain model.
        let gltf = match compressed_copy(&host, &url) {
            Some((copy, ktx2)) => match parse_model(&host, &copy, Some(&ktx2)).await {
                Ok(gltf) => gltf,
                Err(_) => parse_model(&host, &url, None).await?,
            },
            None => parse_model(&host, &url, None).await?,
        };
        // A Persona carries every face; until a look picks one, only the first shows.
        for node in nodes(&get(&gltf, "scene")) {
            let name = string(&get(&get(&node, "material"), "name"));
            if name.starts_with("M_Head_") && !name.ends_with('1') {
                set(&node, "visible", &false.into());
            }
        }
        super::beards::install(&host, &get(&gltf, "scene"), &url).await;
        let _ = install_relief_async(host, get(&gltf, "scene"), url).await;
        Ok(gltf)
    })
}

/// The `.k.json` copy of a model and the host's KTX2 transcoder, when this device's quality asks
/// for compressed maps and the build has the copy.
fn compressed_copy(host: &JsValue, url: &str) -> Option<(String, JsValue)> {
    let copy = format!("{}.k.json", url.strip_suffix(".json")?);
    let (has, ktx2) = (get(host, "hasFile"), get(host, "ktx2"));
    if !super::platform::compressed_textures() || !has.is_function() || !ktx2.is_function() {
        return None;
    }
    let binary = format!("{}.glb", copy.strip_suffix(".json")?);
    if !boolean(&invoke(&has, &[copy.clone().into()])) && !boolean(&invoke(&has, &[binary.into()])) {
        return None;
    }
    let ktx2 = invoke(&ktx2, &[]);
    present(&ktx2).then_some((copy, ktx2))
}
/// The plain `.glb` the build ships next to a model's base64 `.json` (a third smaller, no decode).
fn binary_copy(host: &JsValue, url: &str) -> Option<String> {
    let glb = format!("{}.glb", url.strip_suffix(".json")?);
    let has = get(host, "hasFile");
    (has.is_function() && boolean(&invoke(&has, &[glb.clone().into()]))).then_some(glb)
}
async fn parse_model(host: &JsValue, url: &str, ktx2: Option<&JsValue>) -> Result<JsValue, JsValue> {
    if let Some(glb) = binary_copy(host, url) {
        let response = await_value(try_call(&global(), "fetch", &[glb.as_str().into()])?).await;
        if let Some(response) = response.ok().filter(|r| boolean(&get(r, "ok"))) {
            let bytes = await_value(try_call(&response, "arrayBuffer", &[])?).await?;
            return parse_bytes(host, bytes, ktx2).await;
        }
    }
    let response = await_value(try_call(&global(), "fetch", &[url.into()])?).await?;
    if !boolean(&get(&response, "ok")) {
        return Err(js_sys::Error::new(&format!("model: {}", num(&get(&response, "status")))).into());
    }
    let bytes = if url.ends_with(".json") {
        let payload = await_value(try_call(&response, "json", &[])?).await?;
        let encoded = try_call(&global(), "atob", &[get(&payload, "data")])?;
        let text = encoded.as_string().unwrap_or_default();
        let bytes = text.chars().map(|c| c as u8).collect::<Vec<_>>();
        Uint8Array::from(bytes.as_slice()).buffer().into()
    } else {
        await_value(try_call(&response, "arrayBuffer", &[])?).await?
    };
    parse_bytes(host, bytes, ktx2).await
}
async fn parse_bytes(host: &JsValue, bytes: JsValue, ktx2: Option<&JsValue>) -> Result<JsValue, JsValue> {
    let loader = construct(&get(host, "GLTFLoader"), &[]);
    call(&loader, "setMeshoptDecoder", &[get(host, "MeshoptDecoder")]);
    if let Some(ktx2) = ktx2 {
        call(&loader, "setKTX2Loader", &[ktx2.clone()]);
    }
    await_value(try_call(&loader, "parseAsync", &[bytes, "".into()])?).await
}

/// The files a battle downloads for its figures and guns, named the way this device's loader asks
/// for them (compressed copy, binary). The base fetches them one by one while the player looks at the
/// menu, so the fight starts from the device cache (load-model.ts prefetchBattle).
#[wasm_bindgen(js_name = sceneBattleFiles)]
pub fn battle_files(host: JsValue) -> Array {
    let base = string(&get(&host, "baseUrl"));
    let models = ["persona-female", "persona-male", "holster"];
    let files = Array::new();
    for name in models.iter().chain(GUN_MODELS.iter().chain(HELD_MODELS).map(|(_, m)| m)) {
        let url = format!("{base}models/{name}.json");
        let picked = compressed_copy(&host, &url).map(|(copy, _)| copy).unwrap_or(url);
        let file = binary_copy(&host, &picked).unwrap_or(picked);
        files.push(&file.into());
    }
    files
}

/// Real gun models by held class (pistol, shotgun, rifle, sniper); a class without one keeps its placeholder boxes.
pub(super) const GUN_MODELS: &[(&str, &str)] = &[
    ("pistol", "pustelga"),
    ("shotgun", "taran"),
    ("rifle", "varyag"),
    ("sniper", "berkut"),
];
/// The knife and grenade in the hand: Tripo models already in the hand mount's frame (`models/knife.glb`,
/// `grenade.glb`). Снаряжение keeps showing their catalogue previews, so they stay out of GUN_MODELS.
const HELD_MODELS: &[(&str, &str)] = &[("knife", "knife"), ("grenade", "grenade")];
/// A real gun whose own origin is not where the hand closes: its scale in the hand, then its shift
/// in the mount (x along the bore, y up). The pistol's centre sat in the fist with the grip back in
/// the wrist; it goes up and forward into the palm, a size up to fill the gloved hand (Art, 2026-11-01).
/// The grenade sat in the plane of the hand, showing through its back; it moves out to the palm side (-z)
/// where the throw's half-closed fingers wrap it (2026-11-02).
const HOLDS: &[(&str, f64, [f64; 3])] =
    &[("pistol", 1.2, [0.03, 0.02, 0.]), ("grenade", 1., [0., -0.01, -0.045])];
pub(super) fn gun_classes() -> Vec<&'static str> {
    GUN_MODELS.iter().chain(HELD_MODELS).map(|(held, _)| *held).collect()
}
#[wasm_bindgen(js_name = sceneLoadGuns)]
pub fn load_guns(host: JsValue, priority: Option<JsValue>) -> Promise {
    future_to_promise(async move {
        let guns = Object::new();
        let wanted = priority.filter(present).map(|value| {
            values(&value).into_iter().map(|name| string(&name)).collect::<Vec<_>>()
        });
        let models: Vec<_> = GUN_MODELS
            .iter()
            .chain(HELD_MODELS)
            .filter(|(held, _)| wanted.as_ref().map_or(true, |classes| classes.iter().any(|name| name == held)))
            .copied()
            .collect();
        let loads: Vec<_> = models
            .iter()
            .map(|(_, model)| {
                await_value(invoke(
                    &get(&host, "loadModel"),
                    &[format!("models/{model}.json").into()],
                ))
            })
            .collect();
        for ((held, _), load) in models.iter().zip(loads) {
            if let Ok(model) = load.await {
                let gun = get(&model, "scene");
                prepare_varyag(&gun);
                set(&guns, held, &gun);
            }
        }
        Ok(guns.into())
    })
}
pub(super) fn prepare_varyag(gun: &JsValue) {
    for child in values(&get(gun, "children")) {
        let data = get(&child, "userData");
        if get(&data, "factory").as_bool() == Some(false) {
            let socket = call(
                gun,
                "getObjectByName",
                &[format!("socket_{}", string(&get(&data, "slot"))).into()],
            );
            if present(&socket) {
                call(&socket, "add", &[child]);
            }
        }
    }
}
#[wasm_bindgen(js_name = sceneGripOf)]
pub fn grip_of(host: JsValue, scene: JsValue, side: String) -> JsValue {
    call(&scene, "updateMatrixWorld", &[true.into()]);
    let bone = |name: &str| {
        call(
            &scene,
            "getObjectByName",
            &[format!("{name}_{side}").into()],
        )
    };
    let hand = bone("hand");
    let mid = bone("middle_01");
    let index = bone("index_01");
    let pinky = bone("pinky_01");
    if [&hand, &mid, &index, &pinky].iter().any(|v| !present(v)) {
        return JsValue::NULL;
    }
    let p = |o: &JsValue| call(o, "getWorldPosition", &[make(&host, "Vector3", &[])]);
    let h = p(&hand);
    let along = call(&p(&mid), "sub", &[h.clone()]);
    let x = call(&call(&along, "clone", &[]), "normalize", &[]);
    let y = call(&p(&index), "sub", &[p(&pinky)]);
    let dot = num(&call(&y, "dot", &[x.clone()]));
    call(&y, "addScaledVector", &[x.clone(), (-dot).into()]);
    call(&y, "normalize", &[]);
    let z = call(
        &make(&host, "Vector3", &[]),
        "crossVectors",
        &[x.clone(), y.clone()],
    );
    let grip = call(&make(&host, "Matrix4", &[]), "makeBasis", &[x, y, z]);
    call(
        &grip,
        "setPosition",
        &[call(&h, "addScaledVector", &[along, 0.55.into()])],
    );
    let inverse = call(
        &call(&get(&hand, "matrixWorld"), "clone", &[]),
        "invert",
        &[],
    );
    call(&inverse, "multiply", &[grip])
}

#[wasm_bindgen]
pub struct RustSceneHand {
    host: JsValue,
    mount: JsValue,
    items: Vec<(String, JsValue)>,
    shown: Option<String>,
    /// The guns that wear the vertical foregrip now; the support hand holds it instead of the forend.
    foregrip: HashSet<String>,
    muzzles: HashMap<String, JsValue>,
    /// The mount's rest turn in the hand, and how far a round's kick still lifts the muzzle (0..1).
    rest: JsValue,
    recoil: f64,
}
#[wasm_bindgen]
impl RustSceneHand {
    #[wasm_bindgen(constructor)]
    pub fn new(
        host: JsValue,
        model: JsValue,
        grip: JsValue,
        material: JsValue,
        options: JsValue,
    ) -> Self {
        let mount = make(&host, "Group", &[]);
        call(
            &grip,
            "decompose",
            &[
                get(&mount, "position"),
                get(&mount, "quaternion"),
                get(&mount, "scale"),
            ],
        );
        call(
            &call(&model, "getObjectByName", &["hand_r".into()]),
            "add",
            &[mount.clone()],
        );
        let guns = get(&options, "guns");
        let layer = num(&get(&options, "layer"));
        let look = get(&options, "look");
        let mut looks: Vec<(JsValue, JsValue)> = vec![];
        let mut items = vec![];
        let boxes: serde_json::Value =
            serde_json::from_str(include_str!("held-boxes.json")).unwrap();
        for held in ["pistol", "rifle", "shotgun", "sniper", "knife", "grenade"] {
            let group = make(&host, "Group", &[]);
            set(&group, "visible", &false.into());
            let gun = get(&guns, held);
            if present(&gun) {
                call(&group, "add", &[call(&gun, "clone", &[])]);
                for node in nodes(&group) {
                    if !boolean(&get(&node, "isMesh")) {
                        continue;
                    }
                    let source = get(&node, "material");
                    let replacement = if let Some((_, replacement)) =
                        looks.iter().find(|(m, _)| Object::is(m, &source))
                    {
                        replacement.clone()
                    } else {
                        let replacement = if present(&look) {
                            invoke(&look, &[source.clone()])
                        } else {
                            source.clone()
                        };
                        looks.push((source, replacement.clone()));
                        replacement
                    };
                    set(&node, "material", &replacement);
                }
                pull_stocks(&host, &group);
                if let Some((_, scale, [x, y, z])) = HOLDS.iter().find(|(gun, ..)| *gun == held) {
                    call(&get(&group, "scale"), "setScalar", &[(*scale).into()]);
                    call(&get(&group, "position"), "set", &[(*x).into(), (*y).into(), (*z).into()]);
                }
            } else {
                for b in boxes[held].as_array().unwrap() {
                    let d = b
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap())
                        .collect::<Vec<_>>();
                    let geometry = make(
                        &host,
                        "BoxGeometry",
                        &[
                            (d[1] - d[0]).into(),
                            (d[3] - d[2]).into(),
                            (d[5] - d[4]).into(),
                        ],
                    );
                    let mesh = make(&host, "Mesh", &[geometry, material.clone()]);
                    call(
                        &get(&mesh, "position"),
                        "set",
                        &[
                            ((d[0] + d[1]) / 2.).into(),
                            ((d[2] + d[3]) / 2.).into(),
                            ((d[4] + d[5]) / 2.).into(),
                        ],
                    );
                    call(&group, "add", &[mesh]);
                }
            }
            for node in nodes(&group) {
                if boolean(&get(&node, "isMesh")) {
                    set(&node, "castShadow", &true.into());
                    set(&node, "frustumCulled", &false.into());
                    call(&get(&node, "layers"), "set", &[layer.into()]);
                }
            }
            call(&mount, "add", &[group.clone()]);
            items.push((held.to_owned(), group));
        }
        let rest = call(&get(&mount, "quaternion"), "clone", &[]);
        let mut hand = Self {
            host,
            mount,
            items,
            shown: None,
            foregrip: HashSet::new(),
            muzzles: HashMap::new(),
            rest,
            recoil: 0.,
        };
        hand.fit(Object::new().into());
        hand
    }
    #[wasm_bindgen(getter)]
    pub fn mount(&self) -> JsValue {
        self.mount.clone()
    }
    /// What the hand shows now ("pistol", "rifle", ... or null).
    #[wasm_bindgen(getter)]
    pub fn held(&self) -> JsValue {
        self.shown.as_deref().map(JsValue::from).unwrap_or(JsValue::NULL)
    }
    /// True when the gun in the hand wears the vertical foregrip.
    #[wasm_bindgen(getter)]
    pub fn foregrip(&self) -> bool {
        self.shown.as_ref().is_some_and(|held| self.foregrip.contains(held))
    }
    pub fn fit(&mut self, mods: JsValue) {
        self.muzzles.clear();
        self.foregrip.clear();
        for (held, group) in &self.items {
            let mut replaced = HashSet::new();
            let all = nodes(group);
            for node in &all {
                let data = get(node, "userData");
                if get(&data, "factory").as_bool() != Some(false) {
                    continue;
                }
                let name = string(&get(&mods, &string(&get(&data, "slot")))).replace('-', "_");
                let visible = !name.is_empty() && name == string(&get(node, "name"));
                set(node, "visible", &visible.into());
                let replacing = string(&get(&data, "replaces"));
                if visible && !replacing.is_empty() {
                    replaced.insert(replacing);
                }
                if visible && name == "rail_kulak" {
                    self.foregrip.insert(held.clone());
                }
            }
            for node in all {
                if get(&get(&node, "userData"), "factory").as_bool() == Some(true) {
                    set(
                        &node,
                        "visible",
                        &(!replaced.contains(&string(&get(&node, "name")))).into(),
                    );
                }
            }
        }
    }
    pub fn hold(&mut self, what: JsValue) {
        self.shown = what.as_string();
        for (held, group) in &self.items {
            set(
                group,
                "visible",
                &(Some(held) == self.shown.as_ref()).into(),
            );
        }
    }
    /// One round's recoil: the muzzle jumps up, then `settle` brings it back.
    pub fn kick(&mut self) {
        self.recoil = 1.;
        self.lift();
    }
    pub fn settle(&mut self, dt: f64) {
        if self.recoil > 0. {
            self.recoil = (self.recoil - dt * 11.).max(0.);
            self.lift();
        }
    }
    fn lift(&self) {
        // The gun points along +x in the mount, so a turn about z raises the muzzle around the grip.
        let turn = make(&self.host, "Quaternion", &[]);
        call(
            &turn,
            "setFromAxisAngle",
            &[
                make(&self.host, "Vector3", &[0.into(), 0.into(), 1.into()]),
                (0.07 * self.recoil).into(),
            ],
        );
        let q = get(&self.mount, "quaternion");
        call(&q, "copy", &[self.rest.clone()]);
        call(&q, "multiply", &[turn]);
    }
    pub fn muzzle(&mut self, out: JsValue) -> bool {
        let Some(what) = self.shown.as_ref() else {
            return false;
        };
        if what == "knife" || what == "grenade" {
            return false;
        }
        let Some((_, group)) = self.items.iter().find(|(held, _)| held == what) else {
            return false;
        };
        let at = self
            .muzzles
            .entry(what.clone())
            .or_insert_with(|| front_of(&self.host, group));
        call(&out, "copy", &[at.clone()]);
        // The muzzle is found in the gun's own frame, which a hold may scale and shift in the mount.
        call(group, "localToWorld", &[out]);
        true
    }
}
/// A stock bought longer than the factory one sticks out through the back of the shoulder, so in
/// the hands it is pulled in to the factory length (the «Опора» stocks are adjustable anyway).
/// A taller butt goes three times its extra height further, so its top clears the collar.
fn pull_stocks(host: &JsValue, gun: &JsValue) {
    call(gun, "updateMatrixWorld", &[true.into()]);
    let into = call(&call(&get(gun, "matrixWorld"), "clone", &[]), "invert", &[]);
    // The back end and the top of a stock in the gun's frame, from each mesh's own box.
    let reach = |stock: &JsValue| {
        let bounds = make(host, "Box3", &[]);
        for node in nodes(stock) {
            if !boolean(&get(&node, "isMesh")) {
                continue;
            }
            let geometry = get(&node, "geometry");
            call(&geometry, "computeBoundingBox", &[]);
            let matrix = call(
                &make(host, "Matrix4", &[]),
                "multiplyMatrices",
                &[into.clone(), get(&node, "matrixWorld")],
            );
            let own = call(&get(&geometry, "boundingBox"), "clone", &[]);
            call(&bounds, "union", &[call(&own, "applyMatrix4", &[matrix])]);
        }
        (num(&get(&get(&bounds, "min"), "x")), num(&get(&get(&bounds, "max"), "y")))
    };
    let stocks: Vec<_> = nodes(gun)
        .into_iter()
        .filter(|node| string(&get(&get(node, "userData"), "slot")) == "stock")
        .collect();
    let Some(factory) = stocks
        .iter()
        .find(|node| get(&get(node, "userData"), "factory").as_bool() == Some(true))
    else {
        return;
    };
    let name = string(&get(factory, "name"));
    let (back, top) = reach(factory);
    for node in &stocks {
        let data = get(node, "userData");
        if get(&data, "factory").as_bool() != Some(false) || string(&get(&data, "replaces")) != name {
            continue;
        }
        let (rear, height) = reach(node);
        let over = back - rear + 3. * (height - top).max(0.);
        if over <= 0.001 {
            continue;
        }
        // Forward along the gun, written in the stock's own parent frame.
        let parent = get(node, "parent");
        let shift = [0., over].map(|x| {
            let p = make(host, "Vector3", &[x.into(), 0.into(), 0.into()]);
            call(&p, "applyMatrix4", &[get(gun, "matrixWorld")]);
            call(&parent, "worldToLocal", &[p])
        });
        call(&get(node, "position"), "add", &[call(&shift[1], "sub", &[shift[0].clone()])]);
    }
}
fn front_of(host: &JsValue, gun: &JsValue) -> JsValue {
    call(gun, "updateMatrixWorld", &[true.into()]);
    let into = call(&call(&get(gun, "matrixWorld"), "clone", &[]), "invert", &[]);
    let p = make(host, "Vector3", &[]);
    let mut front = vec![];
    let mut max = f64::NEG_INFINITY;
    for node in nodes(gun) {
        if !boolean(&get(&node, "isMesh")) || !boolean(&get(&node, "visible")) {
            continue;
        }
        let geometry = get(&node, "geometry");
        call(&geometry, "computeBoundingBox", &[]);
        let bounds = get(&geometry, "boundingBox");
        let matrix = call(
            &make(host, "Matrix4", &[]),
            "multiplyMatrices",
            &[into.clone(), get(&node, "matrixWorld")],
        );
        for ix in ["min", "max"] {
            for iy in ["min", "max"] {
                for iz in ["min", "max"] {
                    call(
                        &p,
                        "set",
                        &[
                            get(&get(&bounds, ix), "x"),
                            get(&get(&bounds, iy), "y"),
                            get(&get(&bounds, iz), "z"),
                        ],
                    );
                    call(&p, "applyMatrix4", &[matrix.clone()]);
                    max = max.max(num(&get(&p, "x")));
                    front.push(call(&p, "clone", &[]));
                }
            }
        }
    }
    let at = make(host, "Vector3", &[]);
    let mut count = 0;
    for p in front {
        if num(&get(&p, "x")) > max - 0.03 {
            call(&at, "add", &[p]);
            count += 1;
        }
    }
    call(&at, "divideScalar", &[(count.max(1) as f64).into()]);
    // A real gun's front face can span a scope or a stock; its muzzle socket marks the bore line.
    let socket = call(gun, "getObjectByName", &["socket_muzzle".into()]);
    if present(&socket) {
        // matrixWorld as updated above: getWorldPosition would refresh the parents and break `into`.
        call(&at, "setFromMatrixPosition", &[get(&socket, "matrixWorld")]);
        call(&at, "applyMatrix4", &[into]);
    }
    call(&at, "setX", &[max.into()]);
    at
}

/// Worn item id to its mesh in the Persona models; items without a mesh of their own wear a close one.
const GEAR: [(&str, &str); 9] = [
    ("head-k6", "Helmet"),
    ("head-sych", "Helmet"),
    ("head-cap", "Cap"),
    ("head-sfera", "Sfera"),
    ("body-plita2", "Vest"),
    ("body-plita4", "Plita4"),
    ("suit-field", "Suit"),
    ("suit-ryvok", "Ryvok"),
    ("boots-bertsy", "Boots"),
];
/// Each slot's meshes; the first is worn for `true` and when the model lacks the item's own.
/// The recoloured Solonchak and Ten left the catalogue (2026-11-02) but are still in the models, so they stay
/// listed here to be hidden.
const SLOTS: [(&str, &[&str]); 4] = [
    ("head", &["Helmet", "Cap", "Sfera"]),
    ("body", &["Vest", "Plita4"]),
    ("suit", &["Suit", "Solonchak", "Ten", "Ryvok"]),
    ("boots", &["Boots"]),
];
/// The kit slot a Persona mesh is worn in.
pub(super) fn gear_slot(name: &str) -> Option<&'static str> {
    SLOTS
        .iter()
        .find(|(_, meshes)| meshes.contains(&name))
        .map(|(slot, _)| *slot)
}
/// The recoloured suits share the field suit's cut, and so its body mask.
fn mask_of(mesh: &str) -> String {
    match mesh {
        "Solonchak" | "Ten" => "_suit".into(),
        _ => format!("_{}", mesh.to_ascii_lowercase()),
    }
}
/// Meshes the plain kit wears; figures are sized by these so other items never change a height.
pub(super) fn plain_kit(name: &str) -> bool {
    !SLOTS.iter().any(|(_, meshes)| {
        meshes[1..]
            .iter()
            .any(|m| name == *m || name.ends_with(&format!("_{m}")))
    })
}
/// The mesh `wear[slot]` puts on: an item id, `true` for the plain kit's, nothing otherwise.
fn worn(model: &JsValue, wear: &JsValue, slot: &str, meshes: &[&str]) -> Option<String> {
    let value = get(wear, slot);
    let mesh = if let Some(id) = value.as_string() {
        GEAR.iter().find(|(item, _)| *item == id).map(|(_, m)| *m)?
    } else if boolean(&value) {
        meshes[0]
    } else {
        return None;
    };
    let has = |m: &str| present(&call(model, "getObjectByName", &[m.into()]));
    [mesh, meshes[0]]
        .into_iter()
        .find(|m| has(m))
        .map(String::from)
}
#[wasm_bindgen(js_name=sceneDress)]
pub fn dress(host: JsValue, model: JsValue, wear: JsValue) {
    let on = SLOTS
        .iter()
        .filter_map(|(slot, meshes)| worn(&model, &wear, slot, meshes))
        .collect::<Vec<_>>();
    // the kit's colours (by item id) for the materials actually worn
    let picked = get(&wear, "colors");
    let colors = object(&[]);
    for (slot, meshes) in SLOTS {
        let item = get(&wear, slot);
        if let (Some(id), Some(mesh)) = (item.as_string(), worn(&model, &wear, slot, meshes)) {
            let color = get(&picked, &id);
            if present(&color) {
                set(&colors, &format!("M_{mesh}"), &color);
            }
        }
    }
    let data = get(&model, "userData");
    set(&data, "cdColors", &colors);
    let head = SLOTS[0].1;
    let hat = on.iter().find(|m| head.contains(&m.as_str())).cloned();
    for node in nodes(&model) {
        if !boolean(&get(&node, "isMesh")) {
            continue;
        }
        let name = string(&get(&node, "name"));
        if SLOTS.iter().any(|(_, meshes)| meshes.contains(&name.as_str())) {
            set(&node, "visible", &on.contains(&name).into());
            // a suit cuts where it pokes through a hard vest, like the body under its clothes
            if !on.contains(&name) {
                continue;
            }
        }
        // Hair squeezed under a hat is `<hair>_<hat>`; the free hair shows when no variant fits.
        if let Some(under) = head.iter().find(|m| name.ends_with(&format!("_{m}"))) {
            set(&node, "visible", &(hat.as_deref() == Some(*under)).into());
            continue;
        }
        let has = |m: &str| present(&call(&model, "getObjectByName", &[format!("{name}_{m}").into()]));
        if head.iter().any(|m| has(m)) {
            set(&node, "visible", &(!hat.as_deref().is_some_and(has)).into());
            continue;
        }
        // a suit's morph per hard vest ("vest", "plita4") tucks the jacket in under it while it is worn
        let dictionary = get(&node, "morphTargetDictionary");
        if present(&dictionary) {
            let influences = get(&node, "morphTargetInfluences");
            for key in Object::keys(&Object::from(dictionary.clone())).iter() {
                let key = string(&key);
                let worn = on.iter().any(|m| m.to_ascii_lowercase() == key);
                let _ = js_sys::Reflect::set(
                    &influences,
                    &get(&dictionary, &key),
                    &(if worn { 1.0 } else { 0.0 }).into(),
                );
            }
        }
        // the tucked suit hides only faces the vest covers whole: a face with one corner under it reached past its edge
        let clothes = gear_slot(&name).is_some();
        let mut geometry = get(&node, "geometry");
        // the body also carries `_<suit>_<vest>`: skin near a tucked jacket, hidden while both are worn
        let pairs = on
            .iter()
            .flat_map(|a| on.iter().map(move |b| format!("{}{}", mask_of(a), mask_of(b))));
        let masks = on
            .iter()
            .map(|m| mask_of(m))
            .chain(pairs)
            .map(|k| call(&geometry, "getAttribute", &[k.into()]))
            .filter(present)
            .collect::<Vec<_>>();
        let attributes = Object::keys(&Object::from(get(&geometry, "attributes")));
        let masked = attributes.iter().any(|k| {
            let k = string(&k);
            k.starts_with('_') && SLOTS.iter().any(|(_, ms)| ms.iter().any(|m| mask_of(m) == k))
        });
        if !masked {
            continue;
        }
        // Clones share geometry: each figure cuts its own copy of the body.
        if !boolean(&get(&get(&node, "userData"), "ownGeometry")) {
            let shared = geometry.clone();
            let full = get(&get(&shared, "userData"), "fullIndex");
            geometry = call(&shared, "clone", &[]);
            set(
                &geometry,
                "userData",
                &object(&[(
                    "fullIndex",
                    if present(&full) { full } else { get(&shared, "index") },
                )]),
            );
            set(&node, "geometry", &geometry);
            set(&get(&node, "userData"), "ownGeometry", &true.into());
        }
        let full = get(&get(&geometry, "userData"), "fullIndex");
        let count = if present(&full) {
            num(&get(&full, "count"))
        } else {
            num(&get(
                &call(&geometry, "getAttribute", &["position".into()]),
                "count",
            ))
        } as u32;
        let mut kept = vec![];
        for i in (0..count).step_by(3) {
            let triangle = [i, i + 1, i + 2].map(|v| {
                if present(&full) {
                    num(&call(&full, "getX", &[v.into()])) as u32
                } else {
                    v
                }
            });
            let hidden = |v: &u32| {
                masks
                    .iter()
                    .any(|m| num(&call(m, "getX", &[(*v).into()])) >= 0.5)
            };
            let cut = if clothes {
                triangle.iter().all(hidden)
            } else {
                triangle.iter().any(hidden)
            };
            if !cut {
                kept.extend(triangle);
            }
        }
        call(
            &geometry,
            "setIndex",
            &[make(
                &host,
                "BufferAttribute",
                &[Uint32Array::from(kept.as_slice()).into(), 1.into()],
            )],
        );
    }
    let look = get(&data, "cdLook");
    if present(&look) {
        super::look::apply(model, look);
    }
}

thread_local! {static RELIEF_PREVIEW:RefCell<Option<bool>>=const{RefCell::new(None)};static MANIFEST:RefCell<Option<Promise>>=const{RefCell::new(None)};}
#[wasm_bindgen(js_name=sceneSetReliefPreview)]
pub fn set_preview(enabled: bool) {
    RELIEF_PREVIEW.with(|v| *v.borrow_mut() = Some(enabled));
}
fn preview() -> bool {
    RELIEF_PREVIEW.with(|v| {
        let mut v = v.borrow_mut();
        *v.get_or_insert_with(|| {
            let search = string(&get(&get(&global(), "location"), "search"));
            !search
                .trim_start_matches('?')
                .split('&')
                .any(|p| p == "relief=0")
        })
    })
}
#[wasm_bindgen(js_name=sceneInstallRelief)]
pub fn install_relief(host: JsValue, root: JsValue, url: String) -> Promise {
    future_to_promise(async move {
        install_relief_async(host, root, url).await?;
        Ok(JsValue::UNDEFINED)
    })
}
async fn install_relief_async(host: JsValue, root: JsValue, url: String) -> Result<(), JsValue> {
    let path = url.split('?').next().unwrap_or("");
    let filename = path.rsplit('/').next().unwrap_or("");
    let name = filename
        .strip_suffix(".json")
        .or_else(|| filename.strip_suffix(".glb"))
        .unwrap_or("");
    if ![
        "varyag",
        "pustelga",
        "taran",
        "berkut",
        "tripo-helmet",
        "tripo-vest",
        "persona-authored",
    ]
    .contains(&name)
    {
        return Ok(());
    }
    let base = string(&get(&host, "baseUrl"));
    let manifest = MANIFEST.with(|m| {
        let mut m = m.borrow_mut();
        m.get_or_insert_with(|| {
            let url = format!("{base}tex/relief/manifest.json");
            future_to_promise(async move {
                let result = async {
                    let r = await_value(try_call(&global(), "fetch", &[url.into()])?).await?;
                    if boolean(&get(&r, "ok")) {
                        await_value(try_call(&r, "json", &[])?).await
                    } else {
                        Ok(JsValue::NULL)
                    }
                }
                .await;
                Ok(result.unwrap_or(JsValue::NULL))
            })
        })
        .clone()
    });
    let catalog = JsFuture::from(manifest).await?;
    if num(&get(&catalog, "version")) != 1. {
        return Ok(());
    }
    let model = get(&get(&catalog, "models"), name);
    if !present(&model) {
        return Ok(());
    }
    let mut materials: Vec<JsValue> = vec![];
    for node in nodes(&root) {
        if !boolean(&get(&node, "isMesh"))
            || !present(&call(
                &get(&node, "geometry"),
                "getAttribute",
                &["uv".into()],
            ))
        {
            continue;
        }
        let material = get(&node, "material");
        let list = if Array::is_array(&material) {
            values(&material)
        } else {
            vec![material]
        };
        for material in list {
            if boolean(&get(&material, "isMeshStandardMaterial"))
                && !materials.iter().any(|m| Object::is(m, &material))
            {
                materials.push(material);
            }
        }
    }
    let pending = Array::new();
    for material in materials {
        let entry = get(&get(&model, "materials"), &string(&get(&material, "name")));
        let relief = string(&get(&entry, "relief"));
        if !boolean(&get(&entry, "eligible"))
            || relief.is_empty()
            || !present(&get(&material, "normalMap"))
            || present(&get(&material, "bumpMap"))
            || !compatible_uv(&host, &material)
        {
            continue;
        }
        let profile = compute(
            &host,
            "graphics.surfaceProfile",
            object(&[
                ("material", entry),
                ("unitsPerMeter", get(&model, "unitsPerMeter")),
            ]),
        );
        if !present(&profile) {
            continue;
        }
        let host = host.clone();
        let url = format!("{base}{relief}");
        pending.push(&future_to_promise(async move {
            let disposed = Rc::new(Cell::new(false));
            let observed = disposed.clone();
            let listener = Closure::wrap(
                Box::new(move |_: JsValue| observed.set(true)) as Box<dyn FnMut(JsValue)>
            );
            call(
                &material,
                "addEventListener",
                &["dispose".into(), listener.as_ref().clone()],
            );
            let loaded = match try_call(
                &make(&host, "TextureLoader", &[]),
                "loadAsync",
                &[url.into()],
            ) {
                Ok(pending) => await_value(pending).await,
                Err(error) => Err(error),
            };
            call(
                &material,
                "removeEventListener",
                &["dispose".into(), listener.as_ref().clone()],
            );
            if let Ok(height) = loaded {
                if disposed.get() {
                    call(&height, "dispose", &[]);
                    return Ok(JsValue::UNDEFINED);
                }
                let source = get(&material, "normalMap");
                for key in ["flipY", "channel"] {
                    set(&height, key, &get(&source, key));
                }
                for key in ["wrapS", "wrapT"] {
                    set(&height, key, &three(&host, "ClampToEdgeWrapping"));
                }
                set(&height, "colorSpace", &three(&host, "NoColorSpace"));
                set(
                    &height,
                    "minFilter",
                    &three(&host, "LinearMipmapLinearFilter"),
                );
                set(&height, "magFilter", &three(&host, "LinearFilter"));
                set(&height, "generateMipmaps", &true.into());
                set(&height, "anisotropy", &4.into());
                set(&height, "needsUpdate", &true.into());
                set(&material, "bumpMap", &height);
                set(&material, "bumpScale", &0.into());
                set(&get(&material, "userData"), "cdSurfaceRelief", &profile);
                set(&material, "needsUpdate", &true.into());
            }
            Ok(JsValue::UNDEFINED)
        }));
    }
    JsFuture::from(Promise::all(&pending)).await?;
    Ok(())
}
fn compatible_uv(host: &JsValue, material: &JsValue) -> bool {
    let reference = get(material, "normalMap");
    if num(&get(&reference, "channel")) != 0.
        || !Object::is(
            &get(material, "normalMapType"),
            &three(host, "TangentSpaceNormalMap"),
        )
    {
        return false;
    }
    call(&reference, "updateMatrix", &[]);
    for slot in [
        "map",
        "normalMap",
        "roughnessMap",
        "metalnessMap",
        "aoMap",
        "emissiveMap",
        "specularColorMap",
        "specularIntensityMap",
    ] {
        let texture = get(material, slot);
        if !present(&texture) {
            continue;
        }
        call(&texture, "updateMatrix", &[]);
        if !Object::is(&get(&texture, "channel"), &get(&reference, "channel"))
            || !Object::is(&get(&texture, "flipY"), &get(&reference, "flipY"))
            || !boolean(&call(
                &get(&texture, "matrix"),
                "equals",
                &[get(&reference, "matrix")],
            ))
        {
            return false;
        }
    }
    boolean(&call(
        &get(&reference, "matrix"),
        "equals",
        &[make(host, "Matrix3", &[])],
    ))
}
#[wasm_bindgen(js_name=sceneReliefHook)]
pub fn relief_hook(host: JsValue, material: JsValue, quality: JsValue) -> JsValue {
    let data = get(&get(&material, "userData"), "cdSurfaceRelief");
    let id = string(&get(&quality, "id"));
    if !preview()
        || !present(&data)
        || !present(&get(&material, "bumpMap"))
        || !present(&get(&material, "normalMap"))
        || id == "low"
        || id == "minimal"
        || !compatible_uv(&host, &material)
    {
        return JsValue::UNDEFINED;
    }
    let program = compute(
        &host,
        "graphics.surfacePom",
        object(&[("quality", id.clone().into())]),
    );
    set(
        &get(&material, "bumpMap"),
        "anisotropy",
        &get(&quality, "anisotropy"),
    );
    let key = format!("{}-{id}", string(&get(&program, "version")));
    let height_map = get(&material, "bumpMap");
    let compile=Closure::wrap(Box::new(move|shader:JsValue|{
        let uniforms=get(&shader,"uniforms");for (key,value) in [("cdSurfaceHeight",height_map.clone()),("cdSurfaceDepth",get(&data,"depthUnits")),("cdSurfaceDetailDepth",num(&get(&data,"detailDepthUnits")).into()),("cdSurfaceFabricDepth",num(&get(&data,"fabricDepthUnits")).into()),("cdSurfaceFabricPeriod",num(&get(&data,"fabricPeriodUnits")).into()),("cdSurfaceMidpoint",get(&data,"midpoint")),("cdSurfaceSteps",get(&program,"steps")),("cdSurfaceRefine",get(&program,"refine")),("cdSurfaceShadowSteps",get(&program,"shadowSteps"))]{set(&uniforms,key,&uniform(value));}
        let vertex=format!("{}{}",include_str!("../../shaders/surface-scale-prefix.glsl"),string(&get(&shader,"vertexShader"))).replacen("#include <begin_vertex>",include_str!("../../shaders/surface-scale-begin.glsl"),1);
        let vertex=vertex.replacen("#include <project_vertex>","#include <project_vertex>\ncdSurfaceLocal=transformed;",1);
        set(&shader,"vertexShader",&vertex.into());
        let mut fragment=string(&get(&shader,"fragmentShader")).replacen("#include <common>",&format!("#include <common>\n{}",string(&get(&program,"common"))),1).replacen("void main() {",include_str!("../../shaders/surface-main.glsl"),1);
        for chunk in ["map_fragment","roughnessmap_fragment","metalnessmap_fragment","aomap_fragment","emissivemap_fragment","lights_physical_fragment"]{fragment=fragment.replacen(&format!("#include <{chunk}>"),&string(&get(&three(&host,"ShaderChunk"),chunk)),1);}
        let normals=format!("{}\nnormal=cdSurfaceDetailNormal(normal);",string(&get(&three(&host,"ShaderChunk"),"normal_fragment_maps")));
        fragment=fragment.replacen("#include <normal_fragment_maps>",&normals,1);
        let lighting=string(&get(&three(&host,"ShaderChunk"),"lights_fragment_begin")).replace("RE_Direct( directLight,","directLight.color *= cdSurfaceLightVisibility(directLight.direction);\nRE_Direct( directLight,");
        fragment=fragment.replacen("#include <lights_fragment_begin>",&lighting,1);
        fragment=replace_regex(&fragment,r"texture2D\(\s*(map|normalMap|roughnessMap|metalnessMap|aoMap|emissiveMap|specularColorMap|specularIntensityMap)\s*,\s*v\w+Uv\s*\)","g","textureGrad( $1, cdSurfaceUv, cdSurfaceDx, cdSurfaceDy )");set(&shader,"fragmentShader",&fragment.into());
    })as Box<dyn FnMut(JsValue)>).into_js_value();
    object(&[("key", key.into()), ("compile", compile)])
}

#[wasm_bindgen(js_name=sceneTune)]
pub fn tune(host: JsValue, material: JsValue, quality: JsValue, extra: JsValue, extra_key: String) {
    let taps = num(&get(&quality, "shadowSamples")).max(5.);
    let snippets = compute(
        &host,
        "motion.materialShaders",
        array(&[taps.into()]).into(),
    );
    let standard = boolean(&get(&material, "isMeshStandardMaterial"));
    let relief = if standard {
        relief_hook(host.clone(), material.clone(), quality.clone())
    } else {
        JsValue::UNDEFINED
    };
    let look = super::look::hook(&material);
    // Materials without a look keep the key they had before looks existed.
    let look_key = if present(&look) {
        format!("-{}", string(&get(&look, "key")))
    } else {
        String::new()
    };
    let key = format!(
        "cd-{taps}-{}-{}{look_key}",
        if present(&extra) {
            extra_key
        } else {
            String::new()
        },
        string(&get(&relief, "key"))
    );
    let compile=Closure::wrap(Box::new(move|shader:JsValue|{
        let chunk=string(&get(&three(&host,"ShaderChunk"),"shadowmap_pars_fragment"));
        let chunk=replace_regex(&chunk,r"float phi = interleavedGradientNoise\( gl_FragCoord\.xy \) \* PI2;\s*shadow = \([\s\S]*?\) \* 0\.2;","",&string(&get(&snippets,"shadow")));
        let mut fragment=string(&get(&shader,"fragmentShader")).replacen("#include <shadowmap_pars_fragment>",&chunk,1);
        if standard{fragment=fragment.replacen("#include <lights_physical_fragment>",&string(&get(&snippets,"specular")),1);}
        set(&shader,"fragmentShader",&fragment.into());
        // The look hook appends to map_fragment, so it runs before relief inlines that chunk.
        if present(&look){invoke(&get(&look,"compile"),&[shader.clone()]);}
        if present(&extra){invoke(&extra,&[shader.clone()]);}if present(&relief){invoke(&get(&relief,"compile"),&[shader]);}
    })as Box<dyn FnMut(JsValue)>).into_js_value();
    set(&material, "onBeforeCompile", &compile);
    set(
        &material,
        "customProgramCacheKey",
        &Closure::wrap(Box::new(move || JsValue::from_str(&key)) as Box<dyn FnMut() -> JsValue>)
            .into_js_value(),
    );
    if standard {
        set(
            &material,
            "envMapIntensity",
            &(if boolean(&get(&quality, "env")) {
                0.35
            } else {
                0.
            })
            .into(),
        );
    }
    set(&material, "needsUpdate", &true.into());
}
