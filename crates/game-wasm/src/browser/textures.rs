//! Atomic texture-tier replacement, stale-request cancellation and GPU resource ownership.
use super::bridge::*;
use js_sys::Promise;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};

const SLOTS: [(&str, &str); 4] = [
    ("map", "color"),
    ("normalMap", "normal"),
    ("roughnessMap", "orm"),
    ("metalnessMap", "orm"),
];
type Maps = [JsValue; 4];
#[derive(Clone)]
struct Material {
    object: JsValue,
    own: Maps,
}
struct TierState {
    disposed: bool,
    three: JsValue,
    materials: Vec<Material>,
    manifest: Promise,
    made: Vec<JsValue>,
    size: u32,
    request: u64,
}

#[wasm_bindgen]
pub struct RustTextureTiers {
    state: Rc<RefCell<TierState>>,
}

#[wasm_bindgen]
impl RustTextureTiers {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, roots: JsValue, model: String) -> Self {
        let materials = Rc::new(RefCell::new(Vec::<Material>::new()));
        for root in values(&roots) {
            let list = materials.clone();
            let visitor = Closure::<dyn FnMut(JsValue)>::new(move |o| {
                let m = get(&o, "material");
                if !boolean(&get(&m, "isMeshStandardMaterial")) || !present(&get(&m, "map")) {
                    return;
                }
                let mut list = list.borrow_mut();
                if list.iter().any(|v| v.object == m) {
                    return;
                }
                let own = std::array::from_fn(|i| get(&m, SLOTS[i].0));
                list.push(Material { object: m, own });
            });
            call(&root, "traverse", &[visitor.as_ref().clone()]);
        }
        let name = model
            .rsplit('/')
            .next()
            .and_then(|v| v.strip_suffix(".json").or_else(|| v.strip_suffix(".glb")))
            .filter(|v| {
                !v.is_empty()
                    && v.bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            });
        let manifest = if let Some(name) = name {
            let fetched = try_call(&global(), "fetch", &[format!("tex/{name}.json").into()]);
            future_to_promise(async move {
                let Ok(p) = fetched else {
                    return Ok(JsValue::NULL);
                };
                let Ok(response) = JsFuture::from(Promise::resolve(&p)).await else {
                    return Ok(JsValue::NULL);
                };
                if !boolean(&get(&response, "ok")) {
                    return Ok(JsValue::NULL);
                }
                let Ok(p) = try_call(&response, "json", &[]) else {
                    return Ok(JsValue::NULL);
                };
                Ok(JsFuture::from(Promise::resolve(&p))
                    .await
                    .unwrap_or(JsValue::NULL))
            })
        } else {
            Promise::resolve(&JsValue::NULL)
        };
        Self {
            state: Rc::new(RefCell::new(TierState {
                disposed: false,
                three: get(&host, "THREE"),
                materials: materials.take(),
                manifest,
                made: vec![],
                size: 1024,
                request: 0,
            })),
        }
    }

    pub fn set(&self, size: u32, anisotropy: f64) -> Promise {
        let state = self.state.clone();
        let (request, manifest) = {
            let mut s = state.borrow_mut();
            if s.disposed {
                return Promise::resolve(&JsValue::UNDEFINED);
            }
            s.request += 1;
            (s.request, s.manifest.clone())
        };
        future_to_promise(async move {
            let manifest = JsFuture::from(manifest).await.unwrap_or(JsValue::NULL);
            let (three, materials) = {
                let s = state.borrow();
                if request != s.request {
                    return Ok(JsValue::UNDEFINED);
                }
                (s.three.clone(), s.materials.clone())
            };
            let mut next = None;
            if size < 1024 {
                next = Some(shrink(&three, &materials, size));
            } else if size > 1024
                && present(&manifest)
                && values(&get(&manifest, "sizes"))
                    .iter()
                    .any(|v| num(v) == size as f64)
            {
                next = fetch_size(&three, &materials, &manifest, size).await.ok();
            }
            let mut s = state.borrow_mut();
            if request != s.request {
                if let Some(maps) = next {
                    dispose_unique(maps.into_iter().flatten());
                }
                return Ok(JsValue::UNDEFINED);
            }
            let old = std::mem::take(&mut s.made);
            if let Some(maps) = &next {
                for tex in maps.iter().flatten().filter(|v| present(v)) {
                    if !s.made.contains(tex) {
                        s.made.push(tex.clone());
                    }
                }
            }
            for (index, material) in materials.iter().enumerate() {
                for (slot, (key, _)) in SLOTS.iter().enumerate() {
                    let mine = next
                        .as_ref()
                        .map(|v| v[index][slot].clone())
                        .unwrap_or(JsValue::UNDEFINED);
                    let tex = if present(&mine) {
                        mine.clone()
                    } else {
                        material.own[slot].clone()
                    };
                    if !present(&tex) {
                        continue;
                    }
                    set(&tex, "anisotropy", &anisotropy.into());
                    if get(&material.object, key) != tex {
                        set(&material.object, key, &tex);
                        set(&material.object, "needsUpdate", &true.into());
                    }
                    if present(&mine) {
                        call(&material.own[slot], "dispose", &[]);
                    }
                }
            }
            dispose_unique(old);
            s.size = if next.is_some() { size } else { 1024 };
            Ok(JsValue::UNDEFINED)
        })
    }

    #[wasm_bindgen(getter)]
    pub fn current(&self) -> u32 {
        self.state.borrow().size
    }

    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
}

impl Drop for RustTextureTiers {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
    }
}

impl TierState {
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.request += 1;
        for material in &self.materials {
            for (slot, (key, _)) in SLOTS.iter().enumerate() {
                if self.made.contains(&get(&material.object, key)) {
                    set(&material.object, key, &material.own[slot]);
                    set(&material.object, "needsUpdate", &true.into());
                }
            }
        }
        dispose_unique(std::mem::take(&mut self.made));
        self.size = 1024;
    }
}

fn copy_with(three: &JsValue, original: &JsValue, image: JsValue) -> JsValue {
    let copy = call(original, "clone", &[]);
    set(
        &copy,
        "source",
        &construct(&get(three, "TextureSource"), &[image]),
    );
    set(&copy, "needsUpdate", &true.into());
    copy
}
fn shrink(three: &JsValue, materials: &[Material], size: u32) -> Vec<Maps> {
    let mut done: Vec<(JsValue, JsValue)> = vec![];
    materials
        .iter()
        .map(|material| {
            std::array::from_fn(|slot| {
                let original = &material.own[slot];
                if !present(original) {
                    return JsValue::UNDEFINED;
                }
                if let Some((_, copy)) = done.iter().find(|(t, _)| t == original) {
                    return copy.clone();
                }
                let image = get(original, "image");
                let canvas = call(&document(), "createElement", &["canvas".into()]);
                let w = (size as f64).min(num(&get(&image, "width")));
                let h = (size as f64).min(num(&get(&image, "height")));
                set(&canvas, "width", &w.into());
                set(&canvas, "height", &h.into());
                let context = call(&canvas, "getContext", &["2d".into()]);
                call(
                    &context,
                    "drawImage",
                    &[image, 0.into(), 0.into(), w.into(), h.into()],
                );
                let copy = copy_with(three, original, canvas);
                done.push((original.clone(), copy.clone()));
                copy
            })
        })
        .collect()
}

async fn fetch_size(
    three: &JsValue,
    materials: &[Material],
    manifest: &JsValue,
    size: u32,
) -> Result<Vec<Maps>, JsValue> {
    let loader = construct(&get(three, "ImageLoader"), &[]);
    // Every unique image starts loading before the first await; shared ORM maps stay shared.
    let mut files: Vec<(String, JsValue, Promise)> = vec![];
    let mut indices = HashMap::<String, usize>::new();
    let mut requested = vec![[None; 4]; materials.len()];
    for (index, material) in materials.iter().enumerate() {
        // One base for every map, or {color, normal, orm} bases (a recolour shares the other maps).
        let entry = get(
            &get(manifest, "materials"),
            &string(&get(&material.object, "name")),
        );
        for (slot, (_, kind)) in SLOTS.iter().enumerate() {
            let base = if entry.is_object() {
                string(&get(&entry, kind))
            } else {
                string(&entry)
            };
            if base.is_empty() || !present(&material.own[slot]) {
                continue;
            }
            let url = format!("{base}.{kind}.{size}.webp");
            let file = if let Some(index) = indices.get(&url) {
                *index
            } else {
                let index = files.len();
                let p = Promise::resolve(&call(&loader, "loadAsync", &[url.clone().into()]));
                files.push((url.clone(), material.own[slot].clone(), p));
                indices.insert(url, index);
                index
            };
            requested[index][slot] = Some(file);
        }
    }
    // allSettled prevents unhandled rejected images and allows cleaning every completed copy.
    let promises: Vec<JsValue> = files.iter().map(|v| v.2.clone().into()).collect();
    let settled = JsFuture::from(Promise::all_settled(&array(&promises))).await?;
    let mut copies = vec![];
    let mut error = None;
    for (index, result) in values(&settled).iter().enumerate() {
        if string(&get(result, "status")) == "fulfilled" {
            copies.push(copy_with(three, &files[index].1, get(result, "value")));
        } else {
            error = Some(get(result, "reason"));
            copies.push(JsValue::UNDEFINED);
        }
    }
    if let Some(error) = error {
        dispose_unique(copies);
        return Err(error);
    }
    Ok(requested
        .into_iter()
        .map(|slots| {
            std::array::from_fn(|i| {
                slots[i]
                    .map(|index| copies[index].clone())
                    .unwrap_or(JsValue::UNDEFINED)
            })
        })
        .collect())
}
fn dispose_unique(textures: impl IntoIterator<Item = JsValue>) {
    let mut disposed = vec![];
    for t in textures {
        if present(&t) && !disposed.contains(&t) {
            call(&t, "dispose", &[]);
            disposed.push(t);
        }
    }
}
