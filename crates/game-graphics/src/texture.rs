use std::f64::consts::TAU;

fn hash(x: i32, y: i32) -> f64 {
    let n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263)) as u32;
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n ^ (n >> 16)) as f64 / u32::MAX as f64
}
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn noise(x: f64, y: f64) -> f64 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let u = smooth(0., 1., x - x.floor());
    let v = smooth(0., 1., y - y.floor());
    lerp(
        lerp(hash(ix, iy), hash(ix + 1, iy), u),
        lerp(hash(ix, iy + 1), hash(ix + 1, iy + 1), u),
        v,
    )
}
fn byte(n: f64) -> u8 {
    n.round().clamp(0., 255.) as u8
}

/// Three coordinated, deterministic maps. Height is authored independently of albedo:
/// dark paint and oxidation do not magically become holes in the metal.
/// Relief R = top height in the 16 mm proxy; G = inward edge chips, at most 3.5 mm.
pub fn generate(kind: &str, size: usize) -> Vec<u8> {
    let mut pixels = vec![0; size * size * 4];
    if kind == "contact" {
        for y in 0..size {
            for x in 0..size {
                let radius = ((x as f64 + 0.5 - size as f64 / 2.)
                    .hypot(y as f64 + 0.5 - size as f64 / 2.)
                    / (size as f64 / 2.))
                    .clamp(0., 1.);
                let alpha = if radius < 0.125 {
                    1.
                } else if radius < 0.35 {
                    lerp(1., 0.65, (radius - 0.125) / 0.225)
                } else {
                    0.65 * (1. - (radius - 0.35) / 0.65)
                };
                let i = (y * size + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[255, 255, 255, byte(alpha * 255.)]);
            }
        }
        return pixels;
    }
    for y in 0..size {
        for x in 0..size {
            let px = (x as f64 + 0.5) * 1024. / size as f64;
            let py = (y as f64 + 0.5) * 1024. / size as f64;
            let broad = noise(px / 150., py / 150.);
            let mottled = noise(px / 28., py / 28.);
            let pits = noise(px / 3., py / 3.);
            let grain = hash(px as i32, py as i32);
            let radius = (px - 512.).hypot(py - 512.) / 512.;
            let edge = if radius > 0.965 - 0.022 * mottled && grain > 0.28 {
                24.
            } else {
                0.
            };
            let stain = smooth(0.35, 0.75, broad);
            let value = 68. + broad * 25. + mottled * 12. + (grain - 0.5) * 6. + edge - stain * 18.;
            let roughness = 186. + broad * 45. + mottled * 14. - edge * 1.4;
            // Small pits and machining marks, with two shallow concentric service grooves.
            let groove = (1. - smooth(0.0013, 0.0044, (radius - 0.718).abs()))
                .max(1. - smooth(0.0012, 0.0036, (radius - 0.95).abs()));
            let pit = smooth(0.68, 0.95, pits) * 0.023;
            let height = (0.985 - pit - groove * 0.12 - (1. - mottled) * 0.008).clamp(0.70, 0.995);
            // Broad, sparse nicks, continuous in XY; no UV seams and no random per-frame noise.
            let chip = smooth(0.64, 0.90, noise(px / 9., py / 9.))
                * (0.35 + 0.65 * noise(px / 30., py / 30.));
            let i = (y * size + x) * 4;
            let pixel = match kind {
                "steel-color" => [
                    byte(value + stain * 3.),
                    byte(value + 1.),
                    byte(value + 2. - stain * 3.),
                    255,
                ],
                "steel-roughness" => [byte(roughness), byte(roughness), byte(roughness), 255],
                "steel-relief" => [byte(height * 255.), byte(chip * 255.), 0, 255],
                _ => [0, 0, 0, 255],
            };
            pixels[i..i + 4].copy_from_slice(&pixel);
        }
    }
    // Real scratches share coordinates across the maps and affect relief by <0.7 mm.
    for i in 0..420 {
        let x = hash(i, 2) * size as f64;
        let y = hash(i, 7) * size as f64;
        let length = (3. + hash(i, 13).powi(3) * 95.) * size as f64 / 1024.;
        let angle = -0.65 + hash(i, 19) * 0.34;
        line(
            &mut pixels,
            size,
            kind,
            x,
            y,
            x + angle.cos() * length,
            y + angle.sin() * length,
            0.5 + hash(i, 31),
        );
    }
    for i in 0..24 {
        let radius = size as f64 * (0.30 + hash(i, 46) * 0.185);
        let start = hash(i, 41) * TAU;
        let arc = 0.04 + hash(i, 53) * 0.3;
        let segments = (radius * arc).ceil().max(2.) as usize;
        for s in 0..segments {
            let a = start + arc * s as f64 / segments as f64;
            let b = start + arc * (s + 1) as f64 / segments as f64;
            line(
                &mut pixels,
                size,
                kind,
                size as f64 / 2. + radius * a.cos(),
                size as f64 / 2. + radius * a.sin(),
                size as f64 / 2. + radius * b.cos(),
                size as f64 / 2. + radius * b.sin(),
                0.7 + hash(i, 49),
            );
        }
    }
    pixels
}

fn line(p: &mut [u8], size: usize, kind: &str, ax: f64, ay: f64, bx: f64, by: f64, width: f64) {
    let min_x = (ax.min(bx) - width).floor().max(0.) as usize;
    let min_y = (ay.min(by) - width).floor().max(0.) as usize;
    let max_x = (ax.max(bx) + width).ceil().min(size as f64 - 1.) as usize;
    let max_y = (ay.max(by) + width).ceil().min(size as f64 - 1.) as usize;
    let dx = bx - ax;
    let dy = by - ay;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let t = (((x as f64 + 0.5 - ax) * dx + (y as f64 + 0.5 - ay) * dy)
                / (dx * dx + dy * dy).max(0.0001))
            .clamp(0., 1.);
            let d = (x as f64 + 0.5 - ax - t * dx).hypot(y as f64 + 0.5 - ay - t * dy);
            let cover = (width * 0.5 + 0.5 - d).clamp(0., 1.);
            let i = (y * size + x) * 4;
            match kind {
                "steel-color" => {
                    for (channel, shade) in [182., 185., 181.].iter().enumerate() {
                        p[i + channel] = byte(lerp(p[i + channel] as f64, *shade, cover * 0.19));
                    }
                }
                "steel-roughness" => {
                    for channel in 0..3 {
                        p[i + channel] = byte(lerp(p[i + channel] as f64, 110., cover * 0.28));
                    }
                }
                "steel-relief" => p[i] = byte(p[i] as f64 - cover * 7.),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_are_deterministic_and_fully_initialized() {
        for kind in ["steel-color", "steel-roughness", "steel-relief", "contact"] {
            let a = generate(kind, 64);
            assert_eq!(a, generate(kind, 64));
            assert_eq!(a.len(), 64 * 64 * 4);
            if kind != "contact" {
                assert!(a.chunks_exact(4).all(|p| p[3] == 255));
            }
        }
    }
    #[test]
    fn relief_is_bounded_inside_its_16mm_proxy() {
        let p = generate("steel-relief", 128);
        assert!(p.chunks_exact(4).all(|v| v[0] > 170 && v[0] < 255));
        assert!(p.chunks_exact(4).any(|v| v[1] > 0));
        assert!(p.chunks_exact(4).any(|v| v[1] == 0));
    }
    #[test]
    fn contact_shadow_fades_to_transparent_at_edge() {
        let p = generate("contact", 64);
        assert!(p[(32 * 64 + 32) * 4 + 3] > 250);
        assert_eq!(p[3], 0);
    }
}
