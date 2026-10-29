//! Authored relief over explicit atlas regions; never interpret paint as depth.
use super::*;
use std::f32::consts::TAU;

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn panel(x: f32, y: f32, r: [f32; 4]) -> f32 {
    smooth(0., 3., x - r[0])
        * smooth(0., 3., r[2] - x)
        * smooth(0., 3., y - r[1])
        * smooth(0., 3., r[3] - y)
}
fn vest_detail(x: f32, y: f32) -> f32 {
    let mut height = 0.;
    for r in [
        [193., 686., 364., 882.],
        [9., 780., 183., 939.],
        [6., 580., 178., 758.],
        [302., 520., 453., 678.],
    ] {
        let weight = panel(x, y, r);
        // Textile weave plus a sewn channel inside each authored panel border.
        let edge = (x - r[0]).min(r[2] - x).min(y - r[1]).min(r[3] - y);
        let channel = (-((edge - 5.0) / 1.15).powi(2)).exp() * -0.00036;
        let stitch = (-((edge - 7.0) / 0.60).powi(2)).exp()
            * 0.00020
            * (0.5 + 0.5 * ((x + y) * TAU / 4.2).cos()).powi(2);
        height += (channel + stitch) * weight;
    }
    // MOLLE tape margins visible in the source atlas. Shallow edge rolls do not
    // duplicate the large pouch/tape volumes that already exist in the mesh.
    for r in [
        [18., 617., 154., 650.],
        [17., 660., 151., 690.],
        [18., 704., 147., 739.],
        [305., 530., 330., 662.],
        [340., 530., 366., 662.],
        [376., 530., 403., 662.],
        [413., 530., 439., 662.],
    ] {
        let weight = panel(x, y, r);
        let short = (r[2] - r[0]).min(r[3] - r[1]);
        let edge = (x - r[0]).min(r[2] - x).min(y - r[1]).min(r[3] - y);
        height += 0.00024 * (-((edge - short * 0.17) / 1.8).powi(2)).exp() * weight;
    }
    height
}
pub fn run(root: &Path) -> Result<()> {
    let dir = root.join("apps/web/public/tex/relief");
    let path = dir.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path)?)?;
    for (model, recovered, range, authored) in [
        ("varyag", 0.0016, 0.0016, false),
        ("tripo-helmet", 0.0011, 0.0011, false),
        ("tripo-vest", 0.0007, 0.0024, true),
    ] {
        for (name, entry) in manifest["models"][model]["materials"]
            .as_object_mut()
            .unwrap()
        {
            if entry.get("detailProfileVersion").is_some() {
                return Err("Detail profile already applied; rebuild source maps first".into());
            }
            let source = image::open(
                root.join("apps/web/public")
                    .join(entry["relief"].as_str().unwrap()),
            )?
            .to_rgba8();
            let (w, h) = source.dimensions();
            let scale = if authored { 2 } else { 1 };
            let (ow, oh) = (w * scale, h * scale);
            let source =
                image::imageops::resize(&source, ow, oh, image::imageops::FilterType::Triangle);
            let mut packed = RgbaImage::new(ow, oh);
            let mut height = GrayImage::new(ow, oh);
            let mut mask = GrayImage::new(ow, oh);
            let mut master: ImageBuffer<Luma<u16>, Vec<u16>> = ImageBuffer::new(ow, oh);
            for (x, y, p) in source.enumerate_pixels() {
                let cover = p[1] as f32 / 255.;
                let edge = smooth(0.0, 0.5, cover);
                let added = if authored {
                    vest_detail(x as f32 / scale as f32, y as f32 / scale as f32) * edge
                } else {
                    0.
                };
                let metres = (p[0] as f32 / 255. - 128. / 255.) * recovered + added;
                let normalized = (128. / 255. + metres / range).clamp(1. / 255., 1.);
                let v = (normalized * 255.).round() as u8;
                let detail = (128. + added / 0.0016 * 255.).round().clamp(1., 255.) as u8;
                let confidence = (cover * 2.0).clamp(0., 1.);
                packed.put_pixel(
                    x,
                    y,
                    Rgba([v, (confidence * 255.).round() as u8, detail, 255]),
                );
                height.put_pixel(x, y, Luma([v]));
                mask.put_pixel(x, y, Luma([(confidence * 255.).round() as u8]));
                master.put_pixel(
                    x,
                    y,
                    Luma([(32768. + (normalized - 128. / 255.) * 65535.)
                        .round()
                        .clamp(1., 65535.) as u16]),
                );
            }
            let prefix = format!("{model}-{name}");
            for (suffix, image) in [
                ("relief", image::DynamicImage::ImageRgba8(packed)),
                ("height", image::DynamicImage::ImageLuma8(height)),
                ("mask", image::DynamicImage::ImageLuma8(mask)),
                ("height16", image::DynamicImage::ImageLuma16(master)),
            ] {
                image.save(dir.join(format!("{prefix}.{suffix}.png")))?;
            }
            entry["sourceDepthMeters"] = entry["depthMeters"].clone();
            entry["depthMeters"] = json!(range);
            entry["maximumDepthMeters"] = json!(range);
            entry["detailDepthMeters"] = json!(if authored { 0.0016 } else { 0. });
            entry["fabricDepthMeters"] = json!(if authored { 0.0004 } else { 0. });
            entry["fabricPeriodMeters"] = json!(if authored { 0.0018 } else { 0. });
            entry["resolution"] = json!([ow, oh]);
            entry["detailProfileVersion"] = json!(1);
            entry["profile"] = json!(if authored {
                "Manually selected textile panels/MOLLE margins: weave, sewing channel and thread. No colour-to-height conversion."
            } else {
                "Reconstructed normal-derived hard-surface relief, recalibrated for readable parallax and actual light visibility."
            });
            println!(
                "{model}/{name}: {ow}x{oh}, height range {range:.4}m, authored textile {authored}"
            );
        }
    }
    // The authored Persona reuses precisely this vest atlas, validated by the
    // original bake; preserve that validation and update only the relief fields.
    let vest = manifest["models"]["tripo-vest"]["materials"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    let alias = &mut manifest["models"]["persona-authored"]["materials"]["M_Vest"];
    for key in [
        "depthMeters",
        "maximumDepthMeters",
        "detailDepthMeters",
        "fabricDepthMeters",
        "fabricPeriodMeters",
        "resolution",
        "detailProfileVersion",
        "profile",
    ] {
        alias[key] = vest[key].clone();
    }
    manifest["runtimePacking"] = json!(
        "R=combined height; G=UV coverage; B=authored-only detail height; A=255. Midpoint=128/255. Source normal is not re-applied as a second bump."
    );
    fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_profiles_are_bounded_and_absent_outside_named_regions() {
        assert_eq!(vest_detail(900., 900.), 0.);
        assert_eq!(vest_detail(500., 500.), 0.);
        let mut lo = 0f32;
        let mut hi = 0f32;
        for y in 0..1024 {
            for x in 0..1024 {
                let v = vest_detail(x as f32, y as f32);
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        assert!(lo >= -0.0006 && hi <= 0.0006);
        assert!(lo < -0.0002 && hi > 0.0001);
    }
}
