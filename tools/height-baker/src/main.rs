//! Reconstruct bounded microrelief from glTF tangent normals, independently per UV island.
//! No albedo luminance is used, and input models are never rewritten.
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{GrayImage, ImageBuffer, Luma, Rgba, RgbaImage};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const LIMIT: u32 = 1024;
const LAMBDA: f32 = 1.0 / (20.0 * 20.0);
const SEAM: usize = 6;
struct Gltf {
    doc: Value,
    bin: Vec<u8>,
}
impl Gltf {
    fn load(path: &Path) -> Result<Self> {
        let input = fs::read(path)?;
        let glb = if input.starts_with(b"glTF") {
            input
        } else {
            let wrapper: Value = serde_json::from_slice(&input)?;
            STANDARD.decode(wrapper["data"].as_str().ok_or("Missing GLB data")?)?
        };
        if &glb[..4] != b"glTF" {
            return Err("Not GLB".into());
        }
        let length = u32::from_le_bytes(glb[12..16].try_into()?) as usize;
        let doc = serde_json::from_slice(&glb[20..20 + length])?;
        let offset = 20 + length;
        let size = u32::from_le_bytes(glb[offset..offset + 4].try_into()?) as usize;
        Ok(Self {
            doc,
            bin: glb[offset + 8..offset + 8 + size].to_vec(),
        })
    }
    fn view(&self, index: usize) -> Result<&[u8]> {
        let v = &self.doc["bufferViews"][index];
        if !v["extensions"].is_null() {
            return Err("Compressed views require explicit decoding".into());
        }
        let start = v["byteOffset"].as_u64().unwrap_or(0) as usize;
        let count = v["byteLength"].as_u64().ok_or("Missing buffer length")? as usize;
        Ok(&self.bin[start..start + count])
    }
    fn accessor(&self, index: usize) -> Result<Vec<f32>> {
        let a = &self.doc["accessors"][index];
        let view = a["bufferView"]
            .as_u64()
            .ok_or("Sparse accessor unsupported")? as usize;
        let bytes = self.view(view)?;
        let dims = match a["type"].as_str().unwrap_or("") {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            _ => return Err("Unsupported accessor shape".into()),
        };
        let component = a["componentType"].as_u64().unwrap();
        let size = match component {
            5121 => 1,
            5123 => 2,
            5125 | 5126 => 4,
            _ => return Err("Unsupported accessor type".into()),
        };
        let stride = self.doc["bufferViews"][view]["byteStride"]
            .as_u64()
            .map(|x| x as usize)
            .unwrap_or(dims * size);
        let offset = a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let mut values = Vec::new();
        for i in 0..a["count"].as_u64().unwrap() as usize {
            for c in 0..dims {
                let start = offset + i * stride + c * size;
                let mut n = match component {
                    5121 => bytes[start] as f32,
                    5123 => u16::from_le_bytes(bytes[start..start + 2].try_into()?) as f32,
                    5125 => u32::from_le_bytes(bytes[start..start + 4].try_into()?) as f32,
                    _ => f32::from_le_bytes(bytes[start..start + 4].try_into()?),
                };
                if a["normalized"].as_bool().unwrap_or(false) {
                    n /= match component {
                        5121 => 255.0,
                        5123 => 65535.0,
                        _ => 1.0,
                    };
                }
                values.push(n);
            }
        }
        Ok(values)
    }
    fn triangles(&self, material: usize) -> Result<Vec<[[f32; 2]; 3]>> {
        let mut triangles = Vec::new();
        for mesh in self.doc["meshes"].as_array().unwrap() {
            for p in mesh["primitives"].as_array().unwrap() {
                if p["material"].as_u64() != Some(material as u64) {
                    continue;
                }
                if p["mode"].as_u64().unwrap_or(4) != 4 {
                    return Err("Only triangle meshes supported".into());
                }
                let uv = self.accessor(
                    p["attributes"]["TEXCOORD_0"]
                        .as_u64()
                        .ok_or("UV0 missing")? as usize,
                )?;
                let indices = if let Some(a) = p["indices"].as_u64() {
                    self.accessor(a as usize)?
                } else {
                    (0..uv.len() / 2).map(|i| i as f32).collect()
                };
                for ix in indices.chunks_exact(3) {
                    let t = std::array::from_fn(|k| {
                        [uv[ix[k] as usize * 2], uv[ix[k] as usize * 2 + 1]]
                    });
                    triangles.push(t);
                }
            }
        }
        Ok(triangles)
    }
    fn normal(&self, material: usize) -> Result<image::RgbImage> {
        let t = self.doc["materials"][material]["normalTexture"]["index"]
            .as_u64()
            .ok_or("No normal texture")? as usize;
        let tex = &self.doc["textures"][t];
        let i = tex["source"]
            .as_u64()
            .or(tex["extensions"]["EXT_texture_webp"]["source"].as_u64())
            .ok_or("No image")? as usize;
        let view = self.doc["images"][i]["bufferView"]
            .as_u64()
            .ok_or("External image unsupported")? as usize;
        Ok(image::load_from_memory(self.view(view)?)?.to_rgb8())
    }
    fn max_dimension(&self) -> f32 {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for mesh in self.doc["meshes"].as_array().unwrap() {
            for p in mesh["primitives"].as_array().unwrap() {
                let a =
                    &self.doc["accessors"][p["attributes"]["POSITION"].as_u64().unwrap() as usize];
                for i in 0..3 {
                    min[i] = min[i].min(a["min"][i].as_f64().unwrap() as f32);
                    max[i] = max[i].max(a["max"][i].as_f64().unwrap() as f32);
                }
            }
        }
        (0..3).map(|i| max[i] - min[i]).fold(0.0, f32::max)
    }
}

fn root(parent: &mut [usize], i: usize) -> usize {
    if parent[i] != i {
        parent[i] = root(parent, parent[i]);
    }
    parent[i]
}
fn raster(triangles: &[[[f32; 2]; 3]], w: usize, h: usize) -> (Vec<u32>, usize) {
    let mut parents: Vec<_> = (0..triangles.len()).collect();
    let mut edges = HashMap::new();
    for (i, t) in triangles.iter().enumerate() {
        for e in 0..3 {
            let q = |p: [f32; 2]| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64);
            let a = q(t[e]);
            let b = q(t[(e + 1) % 3]);
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(other) = edges.insert(key, i) {
                let r = root(&mut parents, i);
                let s = root(&mut parents, other);
                parents[r] = s;
            }
        }
    }
    let mut labels = vec![0; w * h];
    let mut islands = HashMap::new();
    for (i, triangle) in triangles.iter().enumerate() {
        let idroot = root(&mut parents, i);
        let next = islands.len() as u32 + 1;
        let id = *islands.entry(idroot).or_insert(next);
        let t = triangle.map(|p| [p[0] * w as f32, p[1] * h as f32]);
        let area =
            (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]);
        if area.abs() < 1e-7 {
            continue;
        }
        let x0 = t
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let x1 = t
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.0)
            .min(w as f32) as usize;
        let y0 = t
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let y1 = t
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.0)
            .min(h as f32) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let b = ((px - t[0][0]) * (t[2][1] - t[0][1])
                    - (py - t[0][1]) * (t[2][0] - t[0][0]))
                    / area;
                let c = ((t[1][0] - t[0][0]) * (py - t[0][1])
                    - (t[1][1] - t[0][1]) * (px - t[0][0]))
                    / area;
                if b >= -1e-5 && c >= -1e-5 && b + c <= 1.00001 {
                    labels[y * w + x] = id;
                }
            }
        }
    }
    (labels, islands.len())
}

fn neighbors(i: usize, w: usize, h: usize) -> [Option<usize>; 4] {
    [
        if i % w > 0 { Some(i - 1) } else { None },
        if i % w + 1 < w { Some(i + 1) } else { None },
        if i >= w { Some(i - w) } else { None },
        if i + w < w * h { Some(i + w) } else { None },
    ]
}
fn seam_mask(labels: &[u32], w: usize, h: usize) -> Vec<f32> {
    let mut dist = vec![SEAM + 1; w * h];
    for i in 0..labels.len() {
        if labels[i] == 0
            || neighbors(i, w, h)
                .iter()
                .any(|n| n.is_none_or(|j| labels[j] != labels[i]))
        {
            dist[i] = 0;
        }
    }
    for depth in 1..=SEAM {
        let prev = dist.clone();
        for i in 0..labels.len() {
            if prev[i] > depth
                && neighbors(i, w, h)
                    .iter()
                    .flatten()
                    .any(|j| prev[*j] == depth - 1)
            {
                dist[i] = depth;
            }
        }
    }
    dist.iter()
        .map(|d| {
            let t = (*d as f32 / SEAM as f32).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        })
        .collect()
}

fn solve(p: &[f32], q: &[f32], labels: &[u32], w: usize, h: usize) -> (Vec<f32>, usize, f64) {
    let active: Vec<_> = (0..labels.len()).filter(|i| labels[*i] != 0).collect();
    let mut rhs = vec![0.0; labels.len()];
    let mut diag = vec![LAMBDA; labels.len()];
    for &i in &active {
        for (j, g) in [
            (if i % w + 1 < w { Some(i + 1) } else { None }, p),
            (if i + w < w * h { Some(i + w) } else { None }, q),
        ] {
            if let Some(j) = j {
                if labels[i] == labels[j] {
                    let value = (g[i] + g[j]) * 0.5;
                    rhs[i] -= value;
                    rhs[j] += value;
                    diag[i] += 1.0;
                    diag[j] += 1.0;
                }
            }
        }
    }
    let apply = |x: &[f32], out: &mut [f32]| {
        for &i in &active {
            let mut s = diag[i] * x[i];
            for j in neighbors(i, w, h).iter().flatten() {
                if labels[*j] == labels[i] {
                    s -= x[*j];
                }
            }
            out[i] = s;
        }
    };
    let dot = |a: &[f32], b: &[f32]| {
        active
            .iter()
            .map(|i| a[*i] as f64 * b[*i] as f64)
            .sum::<f64>()
    };
    let mut x = vec![0.0; labels.len()];
    let mut r = rhs.clone();
    let mut z = vec![0.0; labels.len()];
    for &i in &active {
        z[i] = r[i] / diag[i];
    }
    let mut d = z.clone();
    let mut ap = vec![0.0; labels.len()];
    let mut rz = dot(&r, &z);
    let initial = dot(&r, &r).sqrt().max(1e-20);
    let mut iterations = 0;
    let mut residual = 1.0;
    for k in 0..400 {
        apply(&d, &mut ap);
        let denominator = dot(&d, &ap);
        if denominator.abs() < 1e-25 {
            break;
        }
        let alpha = (rz / denominator) as f32;
        for &i in &active {
            x[i] += alpha * d[i];
            r[i] -= alpha * ap[i];
        }
        residual = dot(&r, &r).sqrt() / initial;
        iterations = k + 1;
        if residual < 1e-5 {
            break;
        }
        for &i in &active {
            z[i] = r[i] / diag[i];
        }
        let next = dot(&r, &z);
        let beta = (next / rz) as f32;
        rz = next;
        for &i in &active {
            d[i] = z[i] + beta * d[i];
        }
    }
    (x, iterations, residual)
}

fn angular(
    p: &[f32],
    q: &[f32],
    height: &[f32],
    mask: &[f32],
    w: usize,
    h: usize,
) -> (f64, f64, usize) {
    let mut base = 0.0;
    let mut reconstructed = 0.0;
    let mut count = 0;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = y * w + x;
            if mask[i] < 0.999
                || mask[i - 1] < 0.999
                || mask[i + 1] < 0.999
                || mask[i - w] < 0.999
                || mask[i + w] < 0.999
            {
                continue;
            }
            let dx = (height[i + 1] - height[i - 1]) * 0.5;
            let dy = (height[i + w] - height[i - w]) * 0.5;
            let len = (p[i] * p[i] + q[i] * q[i] + 1.0).sqrt();
            let other = (dx * dx + dy * dy + 1.0).sqrt();
            base += (1.0 / len).clamp(-1.0, 1.0).acos() as f64;
            reconstructed += ((p[i] * dx + q[i] * dy + 1.0) / (len * other))
                .clamp(-1.0, 1.0)
                .acos() as f64;
            count += 1;
        }
    }
    (
        base / count.max(1) as f64 * 180.0 / std::f64::consts::PI,
        reconstructed / count.max(1) as f64 * 180.0 / std::f64::consts::PI,
        count,
    )
}

fn bake(
    glb: &Gltf,
    model: &str,
    mi: usize,
    out: &Path,
    depth: f32,
    eligible: bool,
) -> Result<Value> {
    let name = glb.doc["materials"][mi]["name"].as_str().unwrap();
    let source = glb.normal(mi)?;
    let source_size = [source.width(), source.height()];
    let scale = (LIMIT as f32 / source.width().max(source.height()) as f32).min(1.0);
    let w = (source.width() as f32 * scale).round() as usize;
    let h = (source.height() as f32 * scale).round() as usize;
    let normal = image::imageops::resize(
        &source,
        w as u32,
        h as u32,
        image::imageops::FilterType::Triangle,
    );
    let triangles = glb.triangles(mi)?;
    let (labels, islands) = raster(&triangles, w, h);
    let mask = seam_mask(&labels, w, h);
    let mut p = vec![0.0; w * h];
    let mut q = vec![0.0; w * h];
    for (i, pixel) in normal.pixels().enumerate() {
        let nx = pixel[0] as f32 / 127.5 - 1.0;
        let ny = pixel[1] as f32 / 127.5 - 1.0;
        let nz = (pixel[2] as f32 / 127.5 - 1.0).max(0.3);
        p[i] = (-nx / nz).clamp(-2.0, 2.0);
        q[i] = (-ny / nz).clamp(-2.0, 2.0);
    }
    // The base geometry owns each island's mean plane. In particular the common
    // encoded neutral normal (128,128,255) must not become an artificial ramp.
    let mut averages: HashMap<u32, (f64, f64, usize)> = HashMap::new();
    for i in 0..w * h {
        if labels[i] != 0 {
            let entry = averages.entry(labels[i]).or_default();
            entry.0 += p[i] as f64;
            entry.1 += q[i] as f64;
            entry.2 += 1;
        }
    }
    for i in 0..w * h {
        if let Some(&(sx, sy, count)) = averages.get(&labels[i]) {
            p[i] -= (sx / count as f64) as f32;
            q[i] -= (sy / count as f64) as f32;
        }
    }
    let interior_count = mask.iter().filter(|m| **m > 0.999).count().max(1);
    let slope_rms = (p
        .iter()
        .zip(&q)
        .zip(&mask)
        .filter(|(_, m)| **m > 0.999)
        .map(|((p, q), _)| (p * p + q * q) as f64)
        .sum::<f64>()
        / interior_count as f64)
        .sqrt();
    // Near-flat normal maps may contain little more than quantisation/compression.
    // Never expand that noise to the same physical relief as deeply embossed metal.
    let calibrated_depth = depth * (slope_rms as f32 / 0.06).min(1.0);
    let (height, iterations, residual) = solve(&p, &q, &labels, w, h);
    let (flat_error, reconstruction_error, evaluated) = angular(&p, &q, &height, &mask, w, h);
    let mut amplitudes: Vec<_> = height
        .iter()
        .enumerate()
        .filter(|(i, _)| mask[*i] > 0.999)
        .map(|(_, v)| v.abs())
        .collect();
    amplitudes.sort_unstable_by(f32::total_cmp);
    let span = amplitudes
        .get((amplitudes.len() as f32 * 0.995) as usize)
        .copied()
        .unwrap_or(1.0)
        .max(0.0001);
    let safe = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let prefix = format!("{model}-{safe}");
    let mut master: ImageBuffer<Luma<u16>, Vec<u16>> = ImageBuffer::new(w as u32, h as u32);
    let mut runtime = GrayImage::new(w as u32, h as u32);
    let mut validity = GrayImage::new(w as u32, h as u32);
    let mut packed = RgbaImage::new(w as u32, h as u32);
    let mut clipping = 0;
    for i in 0..w * h {
        // Midpoint has a precise neutral code in both output formats. The seam mask
        // is separate, so a material can reject displaced reads into another island.
        let z = (height[i] / span).clamp(-1.0, 1.0) * mask[i];
        if height[i].abs() > span && mask[i] > 0.999 {
            clipping += 1;
        }
        let v = (128.0 + z * 127.0).round().clamp(1.0, 255.0) as u8;
        let m = (mask[i] * 255.0).round() as u8;
        let (x, y) = ((i % w) as u32, (i / w) as u32);
        master.put_pixel(
            x,
            y,
            Luma([(32768.0 + z * 32767.0).round().clamp(1.0, 65535.0) as u16]),
        );
        runtime.put_pixel(x, y, Luma([v]));
        validity.put_pixel(x, y, Luma([m]));
        packed.put_pixel(x, y, Rgba([v, m, 128, 255]));
    }
    runtime.save(out.join(format!("{prefix}.height.png")))?;
    master.save(out.join(format!("{prefix}.height16.png")))?;
    validity.save(out.join(format!("{prefix}.mask.png")))?;
    packed.save(out.join(format!("{prefix}.relief.png")))?;
    let makepath = |suffix: &str| format!("tex/relief/{prefix}.{suffix}.png");
    println!(
        "{model}/{name}: {w}x{h}, {islands} islands; CG {iterations}, residual {residual:.7}; normal error flat {flat_error:.3}deg -> integrated {reconstruction_error:.3}deg ({evaluated}px)"
    );
    Ok(
        json!({"height":makepath("height"),"height16":makepath("height16"),"mask":makepath("mask"),"relief":makepath("relief"),"midpoint":128.0/255.0,"depthMeters":calibrated_depth,"maximumDepthMeters":depth,"eligible":eligible,
        "sourceMaterial":name,"sourceResolution":source_size,"resolution":[w,h],"uvSet":0,"uvIslands":islands,"seamFadePixels":SEAM,
        "solver":{"method":"screened Poisson, Jacobi-preconditioned conjugate gradient","screeningLambda":LAMBDA,"maxSlope":2.0,"iterations":iterations,"relativeResidual":residual,"normalConvention":"glTF tangent-space; dH/du=-Nx/Nz, dH/dv=-Ny/Nz; texture row v increases downward","meanIslandSlopeRemoved":true,"robustHeightHalfRangePixels":span},
        "validation":{"interiorPixels":evaluated,"flatNormalMeanErrorDegrees":flat_error,"integratedNormalMeanErrorDegrees":reconstruction_error,"detrendedNormalSlopeRms":slope_rms,"uvCoveredPixels":labels.iter().filter(|l|**l!=0).count(),"clippedInteriorPixels":clipping,"sourceNormalsAreNotGuaranteedIntegrable":true},
        "scaleNote":"Conservative artistic microrelief amplitude; absolute depth is not recoverable from a normal map alone. Large geometry unchanged."}),
    )
}

fn run() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let out = root.join("apps/web/public/tex/relief");
    fs::create_dir_all(&out)?;
    let mut models = serde_json::Map::new();
    for (model, depth, eligible, physical) in [
        ("varyag", 0.00035, true, 0.790000066),
        ("tripo-helmet", 0.00030, true, 0.30),
        ("tripo-vest", 0.00070, true, 0.60),
        ("tripo-suit-male", 0.00060, false, 1.55),
    ] {
        let glb = Gltf::load(&root.join(format!("apps/web/public/preview/{model}.json")))?;
        let mut materials = serde_json::Map::new();
        for mi in 0..glb.doc["materials"].as_array().unwrap().len() {
            let item = bake(&glb, model, mi, &out, depth, eligible)?;
            materials.insert(
                glb.doc["materials"][mi]["name"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                item,
            );
        }
        models.insert(model.to_owned(),json!({"source":format!("preview/{model}.json"),"sourceMaxDimension":glb.max_dimension(),"intendedMaxDimensionMeters":physical,"unitsPerMeter":glb.max_dimension()/physical,"physicalDimensionEstimated":model!="varyag","materials":materials}));
    }
    // Fitted copies may share atlas coordinates, but do not assume that from names.
    // Validate their actual UV coverage and decoded normal pixels before aliasing.
    let fitted = Gltf::load(&root.join("apps/web/public/preview/persona-authored.json"))?;
    let mut aliases = serde_json::Map::new();
    for (source_model, target_name) in [("tripo-helmet", "M_Helmet"), ("tripo-vest", "M_Vest")] {
        let original =
            Gltf::load(&root.join(format!("apps/web/public/preview/{source_model}.json")))?;
        let Some(mi) = fitted.doc["materials"]
            .as_array()
            .unwrap()
            .iter()
            .position(|m| m["name"] == target_name)
        else {
            continue;
        };
        let test = verify_alias(&original, &fitted, mi)?;
        println!("persona-authored/{target_name}: alias validation {test}");
        if test["pass"] == true {
            let mut data = models[source_model]["materials"]
                .as_object()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .clone();
            data["aliasValidation"] = test;
            aliases.insert(target_name.to_owned(), data);
        }
    }
    if !aliases.is_empty() {
        models.insert("persona-authored".into(),json!({"source":"preview/persona-authored.json","unitsPerMeter":1.0,"physicalDimensionEstimated":false,"materials":aliases}));
    }
    let manifest = json!({"version":1,"method":"Normal-derived bounded microrelief, per UV island. Not an authored high-poly displacement bake.","models":models});
    fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    Ok(())
}
fn verify_alias(source: &Gltf, target: &Gltf, mi: usize) -> Result<Value> {
    let w = 512;
    let h = 512;
    let a = image::imageops::resize(
        &source.normal(0)?,
        w as u32,
        h as u32,
        image::imageops::FilterType::Triangle,
    );
    let b = image::imageops::resize(
        &target.normal(mi)?,
        w as u32,
        h as u32,
        image::imageops::FilterType::Triangle,
    );
    let (sa, _) = raster(&source.triangles(0)?, w, h);
    let (ta, _) = raster(&target.triangles(mi)?, w, h);
    let mut tested = 0;
    let mut overlap = 0;
    let mut angle = 0.0;
    for i in 0..w * h {
        if ta[i] == 0 {
            continue;
        }
        tested += 1;
        if sa[i] == 0 {
            continue;
        }
        overlap += 1;
        let n = |rgb: image::Rgb<u8>| {
            let v = rgb.0.map(|c| c as f64 / 127.5 - 1.0);
            let len = (v.iter().map(|v| v * v).sum::<f64>()).sqrt();
            v.map(|v| v / len.max(1e-8))
        };
        let av = n(*a.get_pixel((i % w) as u32, (i / w) as u32));
        let bv = n(*b.get_pixel((i % w) as u32, (i / w) as u32));
        angle += av
            .iter()
            .zip(bv)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .clamp(-1.0, 1.0)
            .acos();
    }
    let coverage = overlap as f64 / tested.max(1) as f64;
    let error = angle / overlap.max(1) as f64 * 180.0 / std::f64::consts::PI;
    Ok(
        json!({"sourceUvCoverage":coverage,"meanDecodedNormalErrorDegrees":error,"testedPixels":tested,"resolution":[w,h],"pass":coverage>0.98&&error<4.0}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("height-baker: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconstructs_integrable_signal() {
        let w = 32;
        let h = 32;
        let labels = vec![1; w * h];
        let mut p = vec![0.0; w * h];
        let mut q = p.clone();
        let mut original = p.clone();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                original[i] = (x as f32 * 0.25).sin() * (y as f32 * 0.2).cos();
                p[i] = 0.25 * (x as f32 * 0.25).cos() * (y as f32 * 0.2).cos();
                q[i] = -0.2 * (x as f32 * 0.25).sin() * (y as f32 * 0.2).sin();
            }
        }
        let (height, _, res) = solve(&p, &q, &labels, w, h);
        let (flat, integrated, _) = angular(&p, &q, &height, &seam_mask(&labels, w, h), w, h);
        assert!(res < 1e-4);
        assert!(integrated < flat * 0.15, "{integrated}/{flat}");
    }
    #[test]
    fn islands_do_not_exchange_height() {
        let w = 16;
        let h = 16;
        let mut labels = vec![1; w * h];
        for y in 0..h {
            for x in 8..w {
                labels[y * w + x] = 2;
            }
        }
        let mut p = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..8 {
                p[y * w + x] = 0.2;
            }
        }
        let (height, _, _) = solve(&p, &vec![0.0; w * h], &labels, w, h);
        for y in 0..h {
            for x in 8..w {
                assert!(height[y * w + x].abs() < 1e-6);
            }
        }
    }
}
