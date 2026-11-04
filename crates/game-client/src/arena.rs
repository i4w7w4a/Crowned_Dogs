//! Scene policy and frame arithmetic. Three.js only applies these values to scene objects.
use crate::{num, text, truth};
use serde_json::{json, Value};

fn depth(d: f64, d7: f64, back: f64) -> f64 {
    d7 * 1.07_f64.powf(d - 7.0) + back
}
fn ramp(dur: f64) -> f64 {
    0.2_f64.min(dur / 3.0)
}
fn travel(t: f64, dur: f64) -> (f64, f64) {
    let r = ramp(dur);
    let v = 1.0 / (dur - r);
    if t < r {
        (v * t * t / (2.0 * r), v * t / r)
    } else if t > dur - r {
        let e = dur - t;
        (1.0 - v * e * e / (2.0 * r), v * e / r)
    } else {
        (v * (t - r / 2.0), v)
    }
}

pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "client.arena.data" => {
            // The knife and the grenade keep the gun's eye point: their own spots slid the body sideways
            // mid-strike and sat the camera in the right shoulder, so the sleeve filled the screen (2026-11-02).
            let mut views = json!({"rifle":{"at":[0,1.57,0.04],"tilt":0},"shotgun":{"at":[0,1.57,0.04],"tilt":0},"sniper":{"at":[0,1.57,0.04],"tilt":0},"pistol":{"at":[0,1.57,0.04],"tilt":0}});
            if let Some(t) = args[0].as_array().filter(|a| a.len() == 4) {
                for v in views.as_object_mut().unwrap().values_mut() {
                    v["at"] = json!([t[0], t[1], t[2]]);
                    v["tilt"] = t[3].clone();
                }
            }
            json!({"views":views,"muzzle":{"size":{"pistol":0.26,"rifle":0.34,"shotgun":0.46,"sniper":0.42},"rounds":{"rifle":4}}})
        }
        "client.arena.calibrate" => {
            let f = &args[0];
            let c = &f["cal"];
            let fp = truth(&args[1]);
            let aspect = num(&f["w"]) / num(&f["h"]);
            let focal = 0.5 / (num(&c["fov"]).to_radians() / 2.0).tan();
            json!({"fn":focal,"aspect":aspect,"d7":1.8*focal/num(&c["height"])*if fp{num(&c["fp"]["near"])}else{1.0},"camH":if fp{&c["fp"]["camH"]}else{&c["camH"]},"hz":num(&c["feet"][1])-num(&c["camH"])*num(&c["height"])/1.8,"kx":(num(&c["feet"][0])-0.5)*aspect/focal,"me":c["me"],"back":if fp{json!(0)}else{c["back"].clone()}})
        }
        "client.arena.move" => {
            let from = num(&args[0]);
            let distance = num(&args[1]);
            let movement = num(&args[2]);
            let previous = num(&args[3]);
            let d7 = num(&args[4]);
            let back = num(&args[5]);
            let to = depth(distance, d7, back);
            let ground = (to - from).abs();
            let projected = truth(&args[6]);
            let requested = movement + num(&args[7]);
            // Range is clamped by combat rules. Blocked advances must not walk the camera through the opponent.
            let actual_movement = if projected && requested.abs() > 1e-9 {
                movement * ((previous - distance) / requested).clamp(0.0, 1.0)
            } else {
                movement
            };
            let mine = if actual_movement != 0.0 {
                depth(previous, d7, back) - depth(previous - actual_movement, d7, back)
            } else {
                0.0
            };
            // On a moving 3D stage, the enemy gait covers its own world displacement.
            // Legacy painted halls retain their camera-relative gait.
            let stride = from - to - if projected { mine } else { 0.0 };
            if ground < 0.01 && actual_movement == 0.0 {
                json!({"to":to,"walk":null,"stride":null,"bob":null})
            } else {
                let ahead = to < from;
                let far = ground.max(mine.abs()).max(stride.abs());
                let cruise = if !ahead {
                    1.5_f64.min(far / 0.45)
                } else if far < 0.8 {
                    far / 0.47
                } else if far < 1.4 {
                    2.5
                } else {
                    3.0
                };
                let dur = 0.2 + far / cruise.max(0.1);
                json!({"to":to,"walk":{"from":from,"to":to,"t":0,"dur":dur,"mine":mine},"stride":if stride.abs()>=0.01{Some(stride)}else{None},"bob":if movement!=0.0{Some(if mine.abs()/(dur-ramp(dur))>2.0{0.035}else{0.02})}else{None}})
            }
        }
        "client.arena.stride" => {
            let ground = num(&args[0]);
            let dur = num(&args[1]);
            let walk = &args[2];
            let run = &args[3];
            let scale = num(&args[4]);
            let cruise = ground.abs() / (dur - ramp(dur));
            let running = !run.is_null()
                && ground > 0.0
                && (walk.is_null()
                    || cruise > (num(&walk["speed"]) * num(&run["speed"])).sqrt() * scale);
            let gait = if running { run } else { walk };
            if gait.is_null() {
                Value::Null
            } else {
                json!({"kind":if running{"run"}else{"walk"},"phase":num(&gait["passing"])-ground/(num(&gait["speed"])*scale)})
            }
        }
        "client.arena.hit" => {
            let mut kick = args[0].clone();
            let zone = text(&args[1]);
            let kind = text(&args[2]);
            let grenade = kind == "grenade";
            kick[zone] =
                json!((num(&kick[zone]) + if kind == "blocked" { 0.45 } else { 1.0 }).min(1.4));
            if grenade {
                kick["body"] = json!(1);
                kick["legs"] = json!(0.8);
            }
            json!({"kick":kick,"zone":if grenade{"legs"}else{zone},"height":if grenade{Some(0.35)}else{None},"size":if grenade{1.6}else{0.32},"color":if kind=="blocked"{0x7fe6f2}else{0xffffff},"life":if grenade{0.4}else{0.18}})
        }
        "client.arena.zoneFallback" => json!([
            0.645,
            match text(&args[0]) {
                "head" => 0.26,
                "body" => 0.34,
                "groin" => 0.42,
                _ => 0.49,
            }
        ]),
        "client.arena.shader" => {
            let kind = text(&args[0]);
            let mut vertex = text(&args[1]).to_string();
            let mut fragment = text(&args[2]).to_string();
            if kind == "mirror" {
                vertex=vertex.replacen("#include <common>","#include <common>\nvarying float vWorldY;",1).replacen("#include <project_vertex>","#include <project_vertex>\nvWorldY = (modelMatrix * vec4(transformed, 1.0)).y;",1);
                fragment=fragment.replacen("#include <common>","#include <common>\nvarying float vWorldY;",1).replacen("#include <normal_fragment_maps>","#include <normal_fragment_maps>\n{ vec3 w = transpose(mat3(viewMatrix)) * normal; w.y = -w.y; normal = normalize(mat3(viewMatrix) * w); }",1).replacen("#include <dithering_fragment>","#include <dithering_fragment>\ngl_FragColor *= clamp(1.0 + vWorldY / 1.1, 0.0, 1.0);",1);
            } else if kind == "wide" {
                vertex="uniform float wideK;\nuniform float wideY;\n".to_owned()+&vertex.replacen("#include <project_vertex>","#include <project_vertex>\n\tgl_Position.xy = mix(vec2(0.0, wideY) * gl_Position.w, gl_Position.xy, wideK);",1);
            }
            json!({"vertex":vertex,"fragment":fragment})
        }
        _ => return Err(format!("Unknown arena operation: {method}")),
    })
}

/// Compact numeric ABI avoids JSON serialization in the render loop. Invalid buffers fail visibly.
pub fn compute(kind: &str, x: &[f64]) -> Result<Vec<f64>, String> {
    if kind.starts_with("arena.lens.") {
        return crate::lens::compute(kind, x);
    }
    let need = match kind {
        "arena.depth" => 3,
        "arena.project" => 5,
        "arena.resize" => 5,
        "arena.scale" => 2,
        "arena.walk" | "arena.walkStage" => 6,
        "arena.pace" => 3,
        "arena.place" => 10,
        "arena.view" => 14,
        "arena.bob" => 6,
        "arena.reactions" => 8,
        "arena.spark" => 4,
        "arena.port" => 7,
        "arena.zone" => 7,
        "arena.screen" => 2,
        "arena.frame" => 2,
        _ => return Err(format!("Unknown arena computation: {kind}")),
    };
    if x.len() != need {
        return Err(format!("{kind} requires {need} values, got {}", x.len()));
    }
    Ok(match kind {
        "arena.depth" => vec![depth(x[0], x[1], x[2])],
        "arena.project" => {
            let n = x[0];
            let half = n * x[3] / 2.0 / x[2];
            vec![
                -half,
                half,
                n * x[1] / x[2],
                -n * (1.0 - x[1]) / x[2],
                n,
                x[4],
            ]
        }
        "arena.resize" => vec![
            0.5_f64.max(x[2].min(x[3]).min((x[4] / (x[0] * x[1]).max(1.0)).sqrt())),
            x[0].round(),
            x[1].round(),
        ],
        "arena.scale" => {
            let tall = x[1] - x[0];
            let scale = if tall > 1.4 && tall < 2.2 {
                1.0
            } else {
                1.8 / tall
            };
            vec![scale, -x[0] * scale]
        }
        "arena.walk" | "arena.walkStage" => {
            let t = (x[2] + x[5]).min(x[3]);
            let (s, v) = travel(t, x[3]);
            let projected = kind == "arena.walkStage";
            let mut result = vec![
                t,
                x[0] + (x[1] - x[0]) * s,
                (x[0] - x[1] - if projected { x[4] } else { 0.0 }) * v,
                (x[4] * v).abs(),
                if t >= x[3] { 1.0 } else { 0.0 },
            ];
            if projected {
                result.extend([x[4] * s, x[4] * v]);
            }
            result
        }
        "arena.pace" => vec![x[0] / (x[1] * x[2])],
        "arena.place" => {
            // depth,kx,me.x,me.y,me.z,back,fp,camera.xyz
            let ex = x[1] * x[0];
            let ez = -x[0];
            let fp = x[6] != 0.0;
            let mz = if fp { x[9] } else { x[4] - x[5] };
            let mx = if fp { x[7] } else { x[2] * mz / x[4] };
            let my = if fp { 0.0 } else { x[3] };
            let mut out = vec![
                ex,
                0.0,
                ez,
                (-ex).atan2(x[0]),
                mx,
                my,
                mz,
                (ex - mx).atan2(ez - mz),
                ex - 4.0,
                7.5,
                ez - 14.0,
                ex,
                0.9,
                ez,
                ex - 2.0,
                3.4,
                ez + 3.2,
                ex,
                1.0,
                ez,
            ];
            out.extend(if fp {
                vec![
                    x[7] + 1.0,
                    x[8] + 1.5,
                    x[9] - 2.5,
                    x[7] + 0.1,
                    x[8] - 0.3,
                    x[9] - 0.5,
                ]
            } else {
                vec![mx - 1.0, 3.4, mz + 3.0, mx, 1.5, mz]
            });
            out.extend([ex - 0.2, 0.12, ez + 0.9]);
            out
        }
        "arena.view" => {
            // current xyz+tilt, target xyz+tilt, initialized, dt, yaw, camera xyz.
            let k = if x[8] != 0.0 {
                1.0 - (-x[9] * 6.0).exp()
            } else {
                1.0
            };
            let a = x[0] + (x[4] - x[0]) * k;
            let b = x[1] + (x[5] - x[1]) * k;
            let c = x[2] + (x[6] - x[2]) * k;
            let tilt = x[3] + (x[7] - x[3]) * k;
            // YXZ Euler = yaw about Y after -tilt about X; z angle stays zero.
            let (s, cx) = (-tilt).sin_cos();
            let (sy, cy) = x[10].sin_cos();
            let y = b * cx - c * s;
            let z = b * s + c * cx;
            vec![
                a,
                b,
                c,
                tilt,
                x[11] - (a * cy + z * sy),
                x[12] - y,
                x[13] - (-a * sy + z * cy),
            ]
        }
        "arena.bob" => {
            let mut phase = x[0] + x[4] * x[3] * std::f64::consts::PI / 0.7;
            let mut k = x[1]
                + (if x[4] > 0.05 && x[5] < 0.0 { 1.0 } else { 0.0 } - x[1])
                    * (1.0 - (-x[3] * 8.0).exp());
            if k < 1e-3 {
                k = 0.0;
                phase = 0.0;
            }
            let s = phase.sin();
            vec![phase, k, 0.012 * k * s, -x[2] * k * s * s]
        }
        "arena.reactions" => {
            let decay = (-x[4] * 6.5).exp();
            let mut out = vec![
                -0.6 * x[0],
                -0.25 * x[0],
                -0.32 * x[1],
                -0.22 * x[1],
                0.42 * x[2],
                0.2 * x[2],
                0.35 * x[3],
                -0.75 * x[3],
            ];
            for v in &x[..4] {
                let d = v * decay;
                out.push(if d < 0.01 { 0.0 } else { d })
            }
            let fall = if x[5] >= 0.0 {
                (x[5] + x[4] / x[7]).min(1.0)
            } else {
                x[5]
            };
            let u = (fall * 1.4).min(1.0);
            let e = u * u * (3.0 - 2.0 * u);
            out.extend([fall, fall * x[7], 1.0 + 0.8 * e, x[6] * 0.3 * e]);
            out
        }
        "arena.spark" => {
            let life = (x[0] - x[3].max(0.0)).max(0.0);
            let u = 1.0 - life / x[1];
            vec![
                life,
                1.0 - u,
                x[2] * (0.6 + 0.8 * u),
                if life > 0.0 { 1.0 } else { 0.0 },
            ]
        }
        "arena.port" => {
            let dx = x[3] - x[0];
            let dy = x[4] - x[1];
            let dz = x[5] - x[2];
            let radius = (dx * dx + dy * dy + dz * dz).sqrt() * 0.45;
            vec![
                x[0] + dx * 0.08 - x[6].sin() * radius,
                x[1] + dy * 0.08,
                x[2] + dz * 0.08 - x[6].cos() * radius,
            ]
        }
        "arena.zone" => {
            if x[6] == 2.0 {
                vec![x[0], x[1] - 0.08, x[2]]
            } else {
                let k = if x[6] == 0.0 {
                    0.45
                } else if x[6] == 1.0 {
                    0.75
                } else {
                    0.15
                };
                vec![
                    x[0] + (x[3] - x[0]) * k,
                    x[1] + (x[4] - x[1]) * k,
                    x[2] + (x[5] - x[2]) * k,
                ]
            }
        }
        "arena.screen" => vec![(x[0] + 1.0) / 2.0, (1.0 - x[1]) / 2.0],
        // A frame never steps back: after a long load the first animation-frame stamp can be older
        // than the timer reset, and a -8 s step left the foe in its bind pose for seconds (Art, 4 Nov).
        "arena.frame" => vec![x[0].clamp(0.0, 0.05), 1.0 - 2.0 * x[1]],
        _ => unreachable!(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_frame_never_steps_back_or_jumps() {
        let dt = |d: f64| compute("arena.frame", &[d, 0.0]).unwrap()[0];
        assert_eq!(dt(-8.0), 0.0);
        assert_eq!(dt(0.016), 0.016);
        assert_eq!(dt(3.0), 0.05);
    }
    #[test]
    fn walk_is_continuous_and_stops_exactly() {
        for duration in [0.3, 0.8, 1.4, 3.0] {
            assert_eq!(travel(0.0, duration), (0.0, 0.0));
            assert_eq!(travel(duration, duration), (1.0, 0.0));
            let mut previous = 0.0;
            for n in 0..=100 {
                let (s, v) = travel(duration * n as f64 / 100.0, duration);
                assert!(s >= previous && v >= 0.0);
                previous = s;
            }
        }
    }
    #[test]
    fn malformed_abi_is_rejected() {
        assert!(compute("arena.walk", &[]).is_err());
        assert!(compute("missing", &[]).is_err());
    }

    #[test]
    fn projected_stage_separates_camera_and_enemy_motion() {
        let d7 = 8.1551128032;
        let start = depth(7.0, d7, 0.45);
        // Our one-step advance travels about half a metre, while the stationary opponent does not walk.
        let own = dispatch("client.arena.move", json!([start, 6, 1, 7, d7, 0.45, true])).unwrap();
        let mine = num(&own["walk"]["mine"]);
        assert!(mine > 0.5 && mine < 0.55);
        assert!(own["stride"].is_null());
        let dur = num(&own["walk"]["dur"]);
        let mid = compute(
            "arena.walkStage",
            &[start, num(&own["to"]), 0.0, dur, mine, dur / 2.0],
        )
        .unwrap();
        assert!(mid[2].abs() < 1e-9);
        assert!(mid[5] > 0.0 && mid[5] < mine);
        let end = compute(
            "arena.walkStage",
            &[start, num(&own["to"]), 0.0, dur, mine, dur + 1.0],
        )
        .unwrap();
        assert_eq!(end[4], 1.0);
        assert!((end[5] - mine).abs() < 1e-9);
        assert_eq!(end[6], 0.0);
        // An enemy-only advance changes range and gait but never moves the environment.
        let enemy = dispatch("client.arena.move", json!([start, 6, 0, 7, d7, 0.45, true])).unwrap();
        assert_eq!(num(&enemy["walk"]["mine"]), 0.0);
        assert!(num(&enemy["stride"]) > 0.5);
        // Equal forward/retreat movement can preserve relative range but must still move both in world space.
        let parallel = dispatch(
            "client.arena.move",
            json!([start, 7, 1, 7, d7, 0.45, true, -1]),
        )
        .unwrap();
        assert!((num(&parallel["stride"]) + mine).abs() < 1e-9);
        let retreat = dispatch(
            "client.arena.move",
            json!([start, 8, -1, 7, d7, 0.45, true]),
        )
        .unwrap();
        assert!(num(&retreat["walk"]["mine"]) < -0.55);
        assert!(retreat["stride"].is_null());
        let rush = dispatch("client.arena.move", json!([start, 4, 3, 7, d7, 0.45, true])).unwrap();
        assert!(num(&rush["walk"]["mine"]) > 1.4);
        let blocked = dispatch(
            "client.arena.move",
            json!([depth(1.0, d7, 0.45), 1, 3, 1, d7, 0.45, true, 0]),
        )
        .unwrap();
        assert!(blocked["walk"].is_null());
        let limited = dispatch(
            "client.arena.move",
            json!([depth(2.0, d7, 0.45), 1, 3, 2, d7, 0.45, true, 0]),
        )
        .unwrap();
        assert!(
            (num(&limited["walk"]["mine"]) - (depth(2.0, d7, 0.45) - depth(1.0, d7, 0.45))).abs()
                < 1e-9
        );
    }
}
