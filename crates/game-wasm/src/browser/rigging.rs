//! Skeleton discovery, bone-local axes and reversible first-person mesh selection.
use super::{bridge::*, kinematics::*};
use js_sys::Uint32Array;
use wasm_bindgen::prelude::*;

const RIG: &[(&str, &[&str])] = &[
    ("hips", &["mixamorigHips", "pelvis"]),
    ("spine", &["mixamorigSpine", "spine_01"]),
    ("spine1", &["mixamorigSpine1", "spine_02"]),
    ("spine2", &["mixamorigSpine2", "spine_03"]),
    ("neck", &["mixamorigNeck", "neck_01"]),
    ("head", &["mixamorigHead", "head"]),
    ("rUpLeg", &["mixamorigRightUpLeg", "thigh_r"]),
    ("rLeg", &["mixamorigRightLeg", "calf_r"]),
    ("lUpLeg", &["mixamorigLeftUpLeg", "thigh_l"]),
    ("lLeg", &["mixamorigLeftLeg", "calf_l"]),
];
thread_local! { static AXES: JsValue = js_sys::WeakMap::new().into(); }
#[wasm_bindgen]
pub fn rig_names() -> JsValue {
    object(
        &RIG.iter()
            .map(|(key, names)| {
                (
                    *key,
                    array(&names.iter().map(|name| (*name).into()).collect::<Vec<_>>()).into(),
                )
            })
            .collect::<Vec<_>>(),
    )
}
#[wasm_bindgen]
pub fn rig_find_bones(
    three: &JsValue,
    model: &JsValue,
    bounds: &JsValue,
) -> Result<JsValue, JsValue> {
    let mut bones = Vec::new();
    // Discover head first: its crown marker belongs to this rig's hierarchy.
    for key in [
        "head", "neck", "spine", "spine1", "spine2", "hips", "rUpLeg", "rLeg", "lUpLeg", "lLeg",
    ] {
        let names = RIG.iter().find(|(name, _)| *name == key).unwrap().1;
        let bone = find(model, names);
        if !present(&bone) {
            return Err(js_sys::Error::new(&format!(
                "the model has no bone {}",
                names.join(" / ")
            ))
            .into());
        }
        let q = world_quat(three, &bone);
        call(&q, "invert", &[]);
        let pitch = vector(three, &[1., 0., 0.]);
        let roll = vector(three, &[0., 0., 1.]);
        call(&pitch, "applyQuaternion", &[q.clone()]);
        call(&roll, "applyQuaternion", &[q]);
        AXES.with(|axes| {
            call(
                axes,
                "set",
                &[bone.clone(), object(&[("pitch", pitch), ("roll", roll)])],
            );
        });
        bones.push((key, bone));
    }
    let head = &bones[0].1;
    let mut top = find(model, &["mixamorigHeadTop_End"]);
    if !present(&top) {
        top = make(three, "Object3D", &[]);
        let p = world_position(three, head);
        call(
            &get(&top, "position"),
            "set",
            &[get(&p, "x"), get(&get(bounds, "max"), "y"), get(&p, "z")],
        );
        call(head, "attach", &[top.clone()]);
    }
    bones.insert(1, ("top", top));
    Ok(object(&bones))
}
#[wasm_bindgen]
pub fn rig_axes_pitch(bone: &JsValue, angle: f64) -> JsValue {
    AXES.with(|axes| {
        call(
            bone,
            "rotateOnAxis",
            &[
                get(&call(axes, "get", &[bone.clone()]), "pitch"),
                angle.into(),
            ],
        )
    })
}
#[wasm_bindgen]
pub fn rig_axes_roll(bone: &JsValue, angle: f64) -> JsValue {
    AXES.with(|axes| {
        call(
            bone,
            "rotateOnAxis",
            &[
                get(&call(axes, "get", &[bone.clone()]), "roll"),
                angle.into(),
            ],
        )
    })
}
/// Turns the neck and head about the world up so the face looks at `target`, `weight` of the way. The mocap
/// stances hold the face 30-40° off the gun line, so a fighter looked past the opponent (Art, 2026-11-02).
/// Runs after the mixer each frame; the next frame's clip pose replaces it.
pub(super) fn look_at(
    three: &JsValue,
    neck: &JsValue,
    head: &JsValue,
    target: &[f64],
    weight: f64,
) {
    let ears = AXES.with(|axes| call(axes, "get", &[head.clone()]));
    // A head that `rig_find_bones` never saw has no ear axis to steer by.
    if weight < 1e-3 || !present(&ears) {
        return;
    }
    let ears = get(&ears, "pitch");
    call(head, "updateWorldMatrix", &[true.into(), false.into()]);
    let mut input = components(&world_quat(three, neck));
    input.extend(components(&world_quat(three, head)));
    input.extend(components(&ears));
    input.extend(components(&world_position(three, head)));
    input.extend(target);
    // The neck takes two fifths; past 75° the clip pose is left to show the rest.
    input.extend([weight, 0.4, 1.3]);
    let q = math("look", &input);
    let turn = quaternion(three);
    upload(&turn, &q[0..4]);
    call(&get(neck, "quaternion"), "multiply", &[turn.clone()]);
    upload(&turn, &q[4..8]);
    call(&get(head, "quaternion"), "multiply", &[turn]);
}
fn arm_bone(name: &str) -> bool {
    [
        "upperarm_",
        "lowerarm_",
        "hand_",
        "thumb_",
        "index_",
        "middle_",
        "ring_",
        "pinky_",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
        || ["Left", "Right"].iter().any(|side| {
            ["Arm", "ForeArm", "Hand"]
                .iter()
                .any(|part| name.starts_with(&format!("mixamorig{side}{part}")))
        })
}
struct Swap {
    mesh: JsValue,
    all: JsValue,
    arms: JsValue,
    shown: bool,
}
#[wasm_bindgen]
pub struct RustArmsOnly {
    swaps: Vec<Swap>,
    disposed: bool,
}
#[wasm_bindgen]
impl RustArmsOnly {
    #[wasm_bindgen(constructor)]
    pub fn new(three: &JsValue, model: &JsValue) -> Self {
        let mut swaps = vec![];
        for mesh in descendants(model)
            .into_iter()
            .filter(|o| boolean(&get(o, "isSkinnedMesh")))
        {
            let geometry = get(&mesh, "geometry");
            let skin = call(&geometry, "getAttribute", &["skinIndex".into()]);
            let weight = call(&geometry, "getAttribute", &["skinWeight".into()]);
            let arm = values(&get(&get(&mesh, "skeleton"), "bones"))
                .iter()
                .map(|bone| f64::from(arm_bone(&string(&get(bone, "name")))))
                .collect::<Vec<_>>();
            let all = get(&geometry, "index");
            let count = num(&get(&skin, "count")) as usize;
            let n = num(&get(
                &if present(&all) {
                    all.clone()
                } else {
                    call(&geometry, "getAttribute", &["position".into()])
                },
                "count",
            )) as usize;
            let weights_at = 3 + count * 4;
            let arms_at = weights_at + count * 4;
            let indices_at = arms_at + arm.len();
            let mut input = vec![0.; indices_at + n];
            input[..3].copy_from_slice(&[count as f64, arm.len() as f64, n as f64]);
            for v in 0..count {
                for k in 0..4 {
                    input[3 + v * 4 + k] = num(&call(
                        &skin,
                        "getComponent",
                        &[(v as u32).into(), (k as u32).into()],
                    ));
                    input[weights_at + v * 4 + k] = num(&call(
                        &weight,
                        "getComponent",
                        &[(v as u32).into(), (k as u32).into()],
                    ));
                }
            }
            input[arms_at..indices_at].copy_from_slice(&arm);
            for i in 0..n {
                input[indices_at + i] = if present(&all) {
                    num(&call(&all, "getX", &[(i as u32).into()]))
                } else {
                    i as f64
                };
            }
            let kept = math("armTriangles", &input)
                .iter()
                .map(|v| *v as u32)
                .collect::<Vec<_>>();
            let arms = if kept.is_empty() {
                JsValue::NULL
            } else {
                make(
                    three,
                    "BufferAttribute",
                    &[Uint32Array::from(kept.as_slice()).into(), 1.into()],
                )
            };
            swaps.push(Swap {
                shown: boolean(&get(&mesh, "visible")),
                mesh,
                all,
                arms,
            });
        }
        Self {
            swaps,
            disposed: false,
        }
    }
    pub fn set(&self, on: bool) {
        if self.disposed {
            return;
        }
        for s in &self.swaps {
            set(
                &s.mesh,
                "visible",
                &(s.shown && (!on || present(&s.arms))).into(),
            );
            call(
                &get(&s.mesh, "geometry"),
                "setIndex",
                &[if on && present(&s.arms) {
                    s.arms.clone()
                } else {
                    s.all.clone()
                }],
            );
        }
    }
    pub fn dispose(&mut self) {
        if !self.disposed {
            self.set(false);
            self.disposed = true;
            self.swaps.clear();
        }
    }
}
impl Drop for RustArmsOnly {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// The player's own figure in battle (first person only): every mesh that holds no arm triangle
/// (head, face, hair, beard, legs, vest, helmet, rigid pieces) leaves the scene for good, so a later
/// look or kit change cannot bring it back into the lens. The skeleton stays: the hands, the gun's
/// grip and the dome around me read its bones. Returns how many meshes went.
pub fn strip_to_arms(three: &JsValue, model: &JsValue) -> usize {
    let keep: Vec<JsValue> = {
        let probe = RustArmsOnly::new(three, model);
        probe.swaps.iter().filter(|s| present(&s.arms)).map(|s| s.mesh.clone()).collect()
    };
    let mut gone = 0;
    for mesh in descendants(model).into_iter().filter(|o| boolean(&get(o, "isMesh"))) {
        if keep.iter().any(|k| js_sys::Object::is(k, &mesh)) {
            continue;
        }
        let parent = get(&mesh, "parent");
        if present(&parent) {
            call(&parent, "remove", &[mesh]);
            gone += 1;
        }
    }
    set(&get(model, "userData"), "cdArmsOnly", &true.into());
    gone
}
