//! Inspect the numerical maps together without altering their pixel data.
use image::{Rgb, RgbImage};
use serde_json::Value;
use std::{fs, path::Path};
fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [31, 4, 4, 4, 4, 4, 31],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 25, 21, 19, 19, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        _ => [0; 7],
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Path::new("apps/web/public");
    let manifest: Value =
        serde_json::from_slice(&fs::read(base.join("tex/relief/manifest.json"))?)?;
    let entries = [
        ("varyag", "M_Varyag", "VARYAG"),
        ("varyag", "M_Sokol", "SOKOL"),
        ("varyag", "M_Tishina", "TISHINA"),
        ("tripo-helmet", "", "HELMET"),
        ("tripo-vest", "", "VEST"),
        ("tripo-suit-male", "", "SUIT"),
    ];
    let mut canvas = RgbImage::from_pixel(1168, 848, Rgb([24, 27, 29]));
    for (i, (model, material, label)) in entries.iter().enumerate() {
        let materials = manifest["models"][model]["materials"].as_object().unwrap();
        let data = if material.is_empty() {
            materials.values().next().unwrap()
        } else {
            &materials[*material]
        };
        let file = data["height"].as_str().unwrap();
        let map = image::open(base.join(file))?.to_rgb8();
        let tile = image::imageops::resize(&map, 368, 368, image::imageops::FilterType::Triangle);
        let mask = image::open(base.join(data["mask"].as_str().unwrap()))?.to_luma8();
        let packed = image::open(base.join(data["relief"].as_str().unwrap()))?.to_rgba8();
        let master = image::open(base.join(data["height16"].as_str().unwrap()))?.to_luma16();
        assert_eq!(map.dimensions(), mask.dimensions());
        assert_eq!(map.dimensions(), packed.dimensions());
        assert_eq!(map.dimensions(), master.dimensions());
        for y in 0..map.height() {
            for x in 0..map.width() {
                let hp = map.get_pixel(x, y)[0];
                let mp = mask.get_pixel(x, y)[0];
                let rp = packed.get_pixel(x, y);
                assert_eq!(rp.0, [hp, mp, 128, 255]);
                if mp == 0 {
                    assert_eq!(hp, 128);
                    assert_eq!(master.get_pixel(x, y)[0], 32768);
                }
            }
        }
        let x = 16 + (i % 3) as u32 * 384;
        let y = 16 + (i / 3) as u32 * 416;
        image::imageops::overlay(&mut canvas, &tile, x as i64, (y + 32) as i64);
        for (k, c) in label.chars().enumerate() {
            for (row, bits) in glyph(c).iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        for sy in 0..2 {
                            for sx in 0..2 {
                                canvas.put_pixel(
                                    x + k as u32 * 12 + col * 2 + sx,
                                    y + row as u32 * 2 + sy,
                                    Rgb([230, 231, 233]),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    fs::create_dir_all("docs/relief")?;
    canvas.save("docs/relief/height-overview.png")?;
    println!("6 material sets verified: packing, dimensions, neutral seams; contact sheet saved.");
    Ok(())
}
