use super::*;
pub fn compute(kind: &str, v: &[f64]) -> Result<Vec<f64>, String> {
    let minimum = match kind {
        "reach" => 31,
        "turn" => 14,
        "worldQuat" => 8,
        "worldBlend" => 9,
        "spin" => 12,
        "grip" => 0,
        "gait" => 2,
        "soften" => 4,
        "settle" => 8,
        "armTriangles" => 3,
        "viewmodel" => 14,
        "viewmodelTransforms" => 42,
        "placeHand" => 49,
        "squareShoulders" => 23,
        "support" => 34,
        "supportArm" => 37,
        "wristPole" => 17,
        "collapse" => 15,
        "holdOut" => 15,
        "flash" => 3,
        "viewerTurn" => 6,
        "cameraDistance" => 3,
        "footprint" => 16,
        "feet" => 25,
        "look" => 20,
        "dust" => 2,
        _ => return Err(format!("Unknown motion computation: {kind}")),
    };
    if v.len() < minimum {
        return Err(format!("{kind}: expected at least {minimum} numbers"));
    }
    Ok(match kind {
        "worldQuat" => (quat(v, 0).conjugate() * quat(v, 4)).to_array().to_vec(),
        "worldBlend" => quat(v, 0).slerp(quat(v, 4), v[8]).to_array().to_vec(),
        "turn" => (quat(v, 10).conjugate() * arc(vec(v, 0), vec(v, 3)) * quat(v, 6))
            .to_array()
            .to_vec(),
        "spin" => (quat(v, 8).conjugate() * DQuat::from_axis_angle(vec(v, 0), v[3]) * quat(v, 4))
            .to_array()
            .to_vec(),
        "reach" => {
            let (a, b, c, target, pole) = (vec(v, 0), vec(v, 3), vec(v, 6), vec(v, 9), vec(v, 12));
            let l1 = a.distance(b);
            let l2 = b.distance(c);
            let direction = target - a;
            let len = direction.length();
            if len < 1e-5 || l1 < 1e-8 {
                return Ok(vec![0.0]);
            }
            let n = direction / len;
            let d = len.max((l1 - l2).abs() + 1e-4).min(l1 + l2 - 1e-4);
            let bend = (pole - a) - n * (pole - a).dot(n);
            if bend.length_squared() < 1e-10 {
                return Ok(vec![0.0]);
            }
            let cos = (l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d);
            let sin = (1.0 - cos * cos).max(0.0).sqrt();
            let delta = arc(
                normalized(b - a),
                normalized(bend.normalize() * (l1 * sin) + n * (l1 * cos)),
            );
            let upper = quat(v, 23).conjugate() * delta * quat(v, 15);
            let b2 = a + delta * (b - a);
            let c2 = a + delta * (c - a);
            let end = a + n * d;
            let lower = (delta * quat(v, 27)).conjugate()
                * arc(normalized(c2 - b2), normalized(end - b2))
                * delta
                * quat(v, 19);
            let mut out = vec![1.0];
            appendq(&mut out, upper);
            appendq(&mut out, lower);
            out
        }
        "grip" => {
            let mut out = vec![];
            for j in v.chunks_exact(13) {
                let target = quat(j, 4) * DQuat::from_axis_angle(vec(j, 8), j[11].to_radians());
                appendq(&mut out, quat(j, 0).slerp(target, j[12]));
            }
            out
        }
        "gait" => {
            let dt = v[0];
            let points = &v[1..];
            let count = points.len() / 6;
            if count == 0 || dt <= 0.0 {
                return Ok(vec![0.0, 0.0]);
            }
            let low = points
                .chunks_exact(6)
                .flat_map(|p| [p[1], p[4]])
                .fold(f64::INFINITY, f64::min);
            let mut velocities = vec![];
            let mut gap = f64::INFINITY;
            let mut passing = 0.0;
            for i in 0..count {
                let a = &points[i * 6..];
                let b = &points[((i + 1) % count) * 6..];
                let side = if a[1] < a[4] { 0 } else { 3 };
                if a[side + 1] < low + 0.03 {
                    velocities.push((a[side + 2] - b[side + 2]) / dt);
                }
                let g = (a[2] - a[5]).abs();
                if g < gap {
                    gap = g;
                    passing = i as f64 * dt;
                }
            }
            velocities.sort_by(f64::total_cmp);
            let speed = velocities.get(velocities.len() / 2).copied().unwrap_or(0.0);
            vec![speed, passing]
        }
        "soften" => {
            let (k, keep, sigma, is_quat) = (v[0] as usize, v[1], v[2], v[3] != 0.0);
            let mut values = v[4..].to_vec();
            if k == 0 || values.len() % k != 0 {
                return Err("Invalid animation stride".into());
            }
            let count = values.len() / k;
            if count < 4 {
                return Ok(values);
            }
            if is_quat {
                for i in 1..count {
                    let dot = (0..4).map(|j| values[i * 4 + j] * values[j]).sum::<f64>();
                    if dot < 0.0 {
                        for j in 0..4 {
                            values[i * 4 + j] = -values[i * 4 + j];
                        }
                    }
                }
            }
            let radius = (sigma * 3.0).ceil() as isize;
            let weights: Vec<_> = (-radius..=radius)
                .map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp())
                .collect();
            let mut mean = vec![0.0; k];
            for sample in values.chunks_exact(k) {
                for j in 0..k {
                    mean[j] += sample[j] / count as f64;
                }
            }
            let mut out = vec![0.0; values.len()];
            for i in 0..count {
                let mut sum = 0.0;
                for offset in -radius..=radius {
                    let at = (i as isize + offset).rem_euclid(count as isize) as usize;
                    let weight = weights[(offset + radius) as usize];
                    sum += weight;
                    for j in 0..k {
                        out[i * k + j] += weight * values[at * k + j];
                    }
                }
                for j in 0..k {
                    out[i * k + j] = mean[j] + keep * (out[i * k + j] / sum - mean[j]);
                }
                if is_quat {
                    let len = (0..4).map(|j| out[i * 4 + j].powi(2)).sum::<f64>().sqrt();
                    let len = if len == 0.0 { 1.0 } else { len };
                    for j in 0..4 {
                        out[i * 4 + j] /= len;
                    }
                }
            }
            out
        }
        "settle" => {
            let mut sum = [0.0; 4];
            for p in v[8..].chunks_exact(5) {
                let dot = (0..4).map(|j| sum[j] * p[j]).sum::<f64>();
                let sign = if dot < 0.0 { -1.0 } else { 1.0 };
                for j in 0..4 {
                    sum[j] += p[j] * sign * p[4];
                }
            }
            let q = DQuat::from_array(sum);
            if q.length_squared() < 1e-8 {
                quat(v, 4).to_array().to_vec()
            } else {
                (quat(v, 0).conjugate() * q.normalize() * quat(v, 4))
                    .to_array()
                    .to_vec()
            }
        }
        "armTriangles" => {
            let vertices = v[0] as usize;
            let bones = v[1] as usize;
            let indices = v[2] as usize;
            let weights_at = 3 + vertices * 4;
            let arms_at = weights_at + vertices * 4;
            let index_at = arms_at + bones;
            if v.len() < index_at + indices {
                return Err("Invalid skin buffer".into());
            }
            let arm: Vec<_> = (0..vertices)
                .map(|i| {
                    let mut best = 0.0;
                    let mut bone = 0usize;
                    for k in 0..4 {
                        let w = v[weights_at + i * 4 + k];
                        if w > best {
                            best = w;
                            bone = v[3 + i * 4 + k] as usize;
                        }
                    }
                    bone < bones && v[arms_at + bone] != 0.0
                })
                .collect();
            let mut kept = vec![];
            for tri in v[index_at..index_at + indices].chunks_exact(3) {
                if tri
                    .iter()
                    .all(|i| arm.get(*i as usize).copied().unwrap_or(false))
                {
                    kept.extend(tri);
                }
            }
            kept
        }
        "viewmodel" => {
            let dt = v[0];
            let on = v[1] != 0.0;
            let aiming = v[2] != 0.0;
            let w = smooth(v[4], if on { 1.0 } else { 0.0 }, dt, 10.0);
            let mut since = if aiming && v[7] == 0.0 { -1.0 } else { v[6] };
            if since >= 0.0 {
                since += dt;
            }
            let raise = aiming && since < 0.45;
            let aim = smooth(
                v[5],
                if raise { 1.0 } else { 0.0 },
                dt,
                if raise { 13.0 } else { 5.0 },
            );
            let kick = v[8] * (-dt * 11.0).exp();
            let time = v[9] + dt;
            let (base, target, kicks) = held(v[3] as usize);
            let eased = ease(aim);
            let target = DVec3::from_array(base).lerp(DVec3::from_array(target), eased);
            let at = if v[13] != 0.0 {
                vec(v, 10).lerp(target, 1.0 - (-dt * 6.0).exp())
            } else {
                target
            };
            let sway = 0.004 * (1.0 - 0.6 * eased);
            let p = DVec3::new(
                at.x + sway * (time * 1.3).sin(),
                -at.y + sway * (time * 1.7 + 1.0).sin() + kicks[1] * kick,
                -at.z + kicks[0] * kick,
            );
            let mut out = vec![w, aim, since, if aiming { 1.0 } else { 0.0 }, kick, time];
            appendv(&mut out, at);
            out.push(if w >= 0.01 { 1.0 } else { 0.0 });
            appendv(&mut out, p);
            out.push(kicks[2] * kick);
            out
        }
        "viewmodelTransforms" => {
            let camera = matrix(v, 0);
            let aim = vec(v, 16);
            let p = camera.transform_point3(vec(v, 19));
            let f = normalized(aim - p);
            let r = normalized(f.cross(DVec3::Y));
            let f = DQuat::from_axis_angle(r, v[22]) * f;
            let u = r.cross(f);
            let scale = vec(v, 23);
            let gun = DMat4::from_cols(
                (f * scale.x).extend(0.0),
                (u * scale.y).extend(0.0),
                (r * scale.z).extend(0.0),
                p.extend(1.0),
            );
            (gun * matrix(v, 26).inverse()).to_cols_array().to_vec()
        }
        "placeHand" => {
            let (_, q, to) = matrix(v, 0).to_scale_rotation_translation();
            let w = v[35];
            let target = vec(v, 16).lerp(to, w);
            let camera = matrix(v, 19);
            // The elbow bends between where a straight wrist puts it and straight down the screen:
            // the first alone swings the forearms across the view, the second alone kinks the wrists.
            // `v[36]` is the arm's side (+1 right, -1 left) times how far out to that side the elbow goes.
            let shoulder = vec(v, 41);
            let axis = normalized(target - shoulder);
            let middle = (shoulder + target) * 0.5;
            let flat = |d: DVec3| normalized(d - axis * d.dot(axis));
            let elbow = target - normalized(q * vec(v, 44)) * v[47];
            let down = camera.transform_vector3(DVec3::new(0.3 * v[36], -0.8, 0.05));
            let bend = normalized(flat(elbow - middle) * (1.0 - v[48]) + flat(down) * v[48]);
            let mut out = target.to_array().to_vec();
            appendv(&mut out, middle + bend * 0.3);
            appendq(&mut out, q);
            out
        }
        "squareShoulders" => {
            // Turns the chest so the shoulder line runs along the camera's right, `v[22]` of the way.
            let mut line = vec(v, 3) - vec(v, 0);
            line.y = 0.0;
            let line = normalized(line);
            let mut right = matrix(v, 6).transform_vector3(DVec3::X);
            right.y = 0.0;
            let right = normalized(right);
            let angle = line.cross(right).y.atan2(line.dot(right)) * v[22];
            let mut out = line.to_array().to_vec();
            appendv(&mut out, DQuat::from_rotation_y(angle) * line);
            out
        }
        "support" => {
            let (at, along, across, fingers, thumb) = support(v[32] as usize, v[33] != 0.0);
            let along = normalized(along);
            let across = normalized(across - along * across.dot(along));
            let frame = DMat4::from_cols(
                along.extend(0.0),
                across.extend(0.0),
                along.cross(across).extend(0.0),
                at.extend(1.0),
            );
            let mut out = (matrix(v, 0) * frame * matrix(v, 16).inverse())
                .to_cols_array()
                .to_vec();
            out.extend([fingers, thumb]);
            out
        }
        "supportArm" => {
            let (_, q, target) = matrix(v, 0).to_scale_rotation_translation();
            let w = v[16];
            let (clavicle, upper, lower, hand) = (vec(v, 17), vec(v, 20), vec(v, 23), vec(v, 26));
            // An arm that would lock straight brings its shoulder forward, as a real one does.
            let from = normalized(upper - clavicle);
            let toward = normalized(target - clavicle);
            let need = upper.distance(target) - (upper.distance(lower) + lower.distance(hand)) * 0.94;
            let axis = from.cross(toward);
            let to = if need > 0.0 && axis.length_squared() > 1e-12 {
                let angle = (need / clavicle.distance(upper))
                    .min(0.6)
                    .min(from.angle_between(toward));
                DQuat::from_axis_angle(axis.normalize(), angle * w) * from
            } else {
                from
            };
            let turn = quat(v, 33);
            let pole = target + turn * DVec3::X * 0.35 - DVec3::Y * 0.6 - turn * DVec3::Z * 0.1;
            let mut out = from.to_array().to_vec();
            appendv(&mut out, to);
            appendv(&mut out, hand.lerp(target, w));
            appendv(&mut out, lower.lerp(pole, w));
            appendq(&mut out, quat(v, 29).slerp(q, w));
            out
        }
        "wristPole" => {
            // A pole that puts the elbow where the forearm runs straight into the hand.
            let (shoulder, lower, hand) = (vec(v, 0), vec(v, 3), vec(v, 6));
            let mut elbow = hand - normalized(quat(v, 9) * vec(v, 13)) * lower.distance(hand);
            // `v[17..20]`, when given, points out of the body on this arm's side: a low-ready wrist
            // put the elbow in the middle of the chest, so it stays at most 3 cm inside the shoulder.
            if v.len() >= 20 {
                let side = normalized(vec(v, 17));
                let inside = -0.03 - (elbow - shoulder).dot(side);
                if inside > 0.0 {
                    elbow += side * inside;
                }
            }
            let pole = elbow + (elbow - (shoulder + hand) * 0.5);
            lower.lerp(pole, v[16]).to_array().to_vec()
        }
        "collapse" => {
            let t = v[0];
            let at = vec(v, 1);
            let start = quat(v, 4);
            let fwd = vec(v, 8);
            let side = vec(v, 11);
            let dir = v[14];
            let knees = ease((t / 0.42).clamp(0.0, 1.0));
            let over = ((t - 0.28) / 0.72).clamp(0.0, 1.0).powi(2);
            let land = if t > 1.0 {
                (-(t - 1.0) * 10.0).exp() * ((t - 1.0) * 30.0).sin() * 0.06
            } else {
                0.0
            };
            let y = at.y + (at.y * 0.62 - at.y) * knees + (0.14 - at.y * 0.62) * over + land * 0.4;
            let mut target = at;
            target.y = y;
            target += fwd * (0.06 * knees + 0.06 * over) + side * (0.34 * over * dir);
            let q = DQuat::from_axis_angle(fwd, -(1.42 + land) * over * dir)
                * DQuat::from_axis_angle(side, 0.18 * knees - 0.1 * over)
                * start;
            let mut out = target.to_array().to_vec();
            appendq(&mut out, q);
            out.extend([
                knees,
                over,
                0.1 * knees * (1.0 - 0.6 * over),
                0.2 * knees,
                0.9 * over * dir,
            ]);
            out
        }
        "holdOut" => {
            let q = quat(v, 0);
            let w = v[4];
            let dir = v[5];
            let fwd = q * DVec3::Z;
            let side = q * DVec3::X;
            let target = vec(v, 6) + fwd * (0.24 * w) + DVec3::Y * (0.02 * w);
            let pole = (vec(v, 12) - DVec3::Y * 0.5 + side * (0.35 * dir) + fwd * 0.15)
                .lerp(vec(v, 9), 1.0 - w);
            let mut out = target.to_array().to_vec();
            appendv(&mut out, pole);
            out
        }
        "flash" => {
            // a step back in time must not relight a spent flash (it once grew over the whole screen)
            let life = (v[0] - v[1].max(0.0)).max(0.0);
            let k = life / 0.07;
            vec![life, k, v[2] * (0.7 + 0.5 * k)]
        }
        "viewerTurn" => {
            let q = quat(v, 0);
            (DQuat::from_axis_angle(DVec3::Z, -v[5]) * DQuat::from_axis_angle(DVec3::Y, v[4]) * q)
                .to_array()
                .to_vec()
        }
        "cameraDistance" => {
            let tan_y = v[0];
            let tan_x = v[1];
            let margin = v[2];
            let dist = v[3..]
                .chunks_exact(3)
                .map(|p| p[2] + (p[0].abs() / tan_x).max(p[1].abs() / tan_y) * margin)
                .fold(0.0, f64::max);
            vec![dist]
        }
        "footprint" => {
            let at = vec(v, 0);
            let dir = vec(v, 3);
            let rest_ankle = vec(v, 6);
            let rest_toe = vec(v, 9);
            let rest_q = quat(v, 12);
            let yaw = dir.x.atan2(dir.z).clamp(-0.7, 0.7);
            let toe = DVec3::new(yaw.sin(), 0.0, yaw.cos());
            let mut rest_dir = rest_toe - rest_ankle;
            rest_dir.y = 0.0;
            let mut at = at;
            at.y = rest_ankle.y;
            let mut out = at.to_array().to_vec();
            appendq(&mut out, arc(normalized(rest_dir), toe) * rest_q);
            appendv(&mut out, toe);
            out
        }
        "feet" => {
            let w = v[0];
            let target = vec(v, 1).lerp(vec(v, 4), 1.0 - w);
            let toe = quat(v, 13) * vec(v, 7);
            let pole = vec(v, 10).lerp(target, 0.5) + toe * 0.6 + DVec3::Y * 0.05;
            let flat = quat(v, 13) * quat(v, 17);
            let mut out = target.to_array().to_vec();
            appendv(&mut out, pole);
            appendq(&mut out, flat);
            out
        }
        "look" => {
            // Neck and head quaternions (world), the head's ear-to-ear axis in its own frame, the head and
            // target positions, weight, neck share and the largest turn. The face's heading is taken across
            // the ears, so a canted cheek-on-stock head reads the same as an upright one. The answer turns
            // both bones about the world up and comes back as each bone's local post-rotation.
            let (neck, head) = (quat(v, 0), quat(v, 4));
            let mut face = (head * vec(v, 8)).cross(DVec3::Y);
            face.y = 0.0;
            let mut to = vec(v, 14) - vec(v, 11);
            to.y = 0.0;
            if face.length_squared() < 1e-9 || to.length_squared() < 1e-9 {
                return Ok(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
            }
            let off = face.cross(to).y.atan2(face.dot(to));
            let turn = off.clamp(-v[19], v[19]) * v[17].clamp(0.0, 1.0);
            let local =
                |q: DQuat, angle: f64| DQuat::from_axis_angle(q.conjugate() * DVec3::Y, angle);
            let mut out = Vec::with_capacity(9);
            appendq(&mut out, local(neck, turn * v[18]));
            appendq(&mut out, local(head, turn * (1.0 - v[18])));
            out.push(off);
            out
        }
        "dust" => {
            let count = v[0] as usize;
            if count > 1_000_000 {
                return Err("Too many dust particles".into());
            }
            let mut state = v[1] as u32;
            let mut next = || {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as f64 / 4294967296.0
            };
            let mut positions = Vec::with_capacity(count * 3);
            let mut seeds = Vec::with_capacity(count * 4);
            for _ in 0..count {
                positions.extend([
                    -4.0 + next() * 8.0,
                    0.1 + next() * 4.6,
                    -16.0 + next() * 15.0,
                ]);
                for _ in 0..4 {
                    seeds.push(next());
                }
            }
            positions.extend(seeds);
            positions
        }
        _ => unreachable!(),
    })
}

/// Where the support hand holds each gun, in the gun's mount (x along the bore, y up): the palm
/// centre, the hand's long and across axes, then how far its fingers and its thumb close.
/// Tuned on close-ups (Art, 2026-11-01): a two-hand wrap on the pistol, the hand far out on the
/// pump and the forends, and the vertical foregrip on the pistol and the rifle when fitted.
fn support(stance: usize, foregrip: bool) -> (DVec3, DVec3, DVec3, f64, f64) {
    // The stocked pistol is the same gun, held in the same places.
    if stance == 4 {
        return support(0, foregrip);
    }
    // Under a barrel or a pump: the fingers wrap across it, tilted `th` and swept forward `ph`.
    let barrel = |at: [f64; 3], th: f64, ph: f64| {
        let along = DVec3::new(ph.sin(), -ph.cos() * th.sin(), ph.cos() * th.cos());
        (DVec3::from(at), along, DVec3::new(0.0, th.cos(), th.sin()).cross(along))
    };
    // Round an upright handle (the foregrip, or the pistol hand's front).
    let handle = |at: [f64; 3], th: f64, ph: f64| {
        let turn = DQuat::from_rotation_y(th) * DQuat::from_rotation_z(ph);
        (DVec3::from(at), turn * DVec3::X, turn * DVec3::Y)
    };
    let ((at, along, across), fingers, thumb) = match (stance, foregrip) {
        (0, false) => (handle([0.0231, -0.0414, -0.0757], -0.179, 0.179), 0.74, -1.0),
        // The pistol sits 1.2 times its size, up and forward in the mount (`HOLDS` in scene_assets.rs).
        (0, true) => (handle([0.153, -0.0352, -0.0349], 0.069, 0.638), 1.2, 1.09),
        (1, true) => (handle([0.3151, -0.022, -0.0266], 0.082, 0.624), 1.3, 0.74),
        (2, _) => (barrel([0.43, -0.014, -0.0391], 0.253, 0.462), 0.82, -0.55),
        (3, _) => (barrel([0.40, 0.0375, -0.0363], 0.331, 0.448), 0.82, -0.84),
        _ => (barrel([0.275, 0.0514, -0.0378], 0.26, 0.427), 0.72, -0.76),
    };
    (at, along, across, fingers, thumb)
}
