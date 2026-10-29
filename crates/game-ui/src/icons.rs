//! Silhouette geometry and SVG composition. The approved drawings are data,
//! while bounds, transforms, attachment placement and masks are computed here.
use serde_json::{Value, json};
use std::sync::OnceLock;

const CUT: f64 = 1.6;
fn catalog() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("icons-catalog.json")).expect("valid icon catalog")
    })
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
fn default_number(v: &Value, default: f64) -> f64 {
    v.as_f64().unwrap_or(default)
}
fn string(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}
fn list(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn round(v: f64) -> f64 {
    (v * 100.0 + 0.5).floor() / 100.0
}
fn numeric(v: &Value) -> String {
    let n = number(v);
    if n == 0.0 { "0".into() } else { n.to_string() }
}
fn points(v: &Value) -> String {
    list(v).iter().map(numeric).collect::<Vec<_>>().join(" ")
}

fn arc(args: &Value) -> Value {
    let [cx, cy, rx, ry, from, to] = std::array::from_fn::<_, 6, _>(|i| number(&args[i]));
    let steps = default_number(&args[6], 16.0) as usize;
    let mut result = Vec::with_capacity((steps + 1) * 2);
    for i in 0..=steps {
        let angle = (from + (to - from) * i as f64 / steps as f64).to_radians();
        result.push(round(cx + rx * angle.cos()));
        result.push(round(cy + ry * angle.sin()));
    }
    json!(result)
}
fn transform_primitive(primitive: &Value, dx: f64, dy: f64, scale: f64) -> Value {
    let mut p = primitive.clone();
    match string(&primitive["k"]) {
        "rect" => {
            p["x"] = json!(round(dx + scale * number(&primitive["x"])));
            p["y"] = json!(round(dy + scale * number(&primitive["y"])));
            for field in ["w", "h", "r"] {
                p[field] = json!(round(scale * number(&primitive[field])));
            }
        }
        "circle" | "ellipse" => {
            p["cx"] = json!(round(dx + scale * number(&primitive["cx"])));
            p["cy"] = json!(round(dy + scale * number(&primitive["cy"])));
            let fields = if primitive["k"] == "circle" {
                vec!["r"]
            } else {
                vec!["rx", "ry"]
            };
            for field in fields {
                p[field] = json!(round(scale * number(&primitive[field])));
            }
        }
        "poly" | "line" => {
            p["pts"] = json!(
                list(&primitive["pts"])
                    .iter()
                    .enumerate()
                    .map(|(i, v)| round(if i % 2 == 0 { dx } else { dy } + scale * number(v)))
                    .collect::<Vec<_>>()
            );
            if primitive["k"] == "line" {
                p["w"] = json!(round(scale * default_number(&primitive["w"], CUT)));
            }
        }
        _ => {}
    }
    p
}
fn place(shape: &Value, dx: f64, dy: f64, scale: f64) -> Value {
    json!({"body":list(&shape["body"]).iter().map(|p|transform_primitive(p,dx,dy,scale)).collect::<Vec<_>>(),
        "cuts":list(&shape["cuts"]).iter().map(|p|transform_primitive(p,dx,dy,scale)).collect::<Vec<_>>()})
}
fn merge(shapes: &[Value]) -> Value {
    json!({"body":shapes.iter().flat_map(|s|list(&s["body"]).iter().cloned()).collect::<Vec<_>>(),
        "cuts":shapes.iter().flat_map(|s|list(&s["cuts"]).iter().cloned()).collect::<Vec<_>>()})
}
fn bounds(shape: &Value) -> [f64; 4] {
    let [mut x0, mut y0, mut x1, mut y1] = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut add = |x: f64, y: f64| {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    };
    for p in list(&shape["body"]) {
        match string(&p["k"]) {
            "rect" => {
                add(number(&p["x"]), number(&p["y"]));
                add(
                    number(&p["x"]) + number(&p["w"]),
                    number(&p["y"]) + number(&p["h"]),
                );
            }
            "circle" => {
                add(
                    number(&p["cx"]) - number(&p["r"]),
                    number(&p["cy"]) - number(&p["r"]),
                );
                add(
                    number(&p["cx"]) + number(&p["r"]),
                    number(&p["cy"]) + number(&p["r"]),
                );
            }
            "ellipse" => {
                add(
                    number(&p["cx"]) - number(&p["rx"]),
                    number(&p["cy"]) - number(&p["ry"]),
                );
                add(
                    number(&p["cx"]) + number(&p["rx"]),
                    number(&p["cy"]) + number(&p["ry"]),
                );
            }
            "poly" | "line" => {
                for pair in list(&p["pts"]).chunks_exact(2) {
                    add(number(&pair[0]), number(&pair[1]));
                }
            }
            _ => {}
        }
    }
    if x0.is_infinite() {
        [0.0, 0.0, 1.0, 1.0]
    } else {
        [x0, y0, x1 - x0, y1 - y0]
    }
}
fn primitive(p: &Value, fill: &str) -> String {
    let n = |field: &str| numeric(&p[field]);
    match string(&p["k"]) {
        "rect" => {
            let radius = if number(&p["r"]) != 0.0 {
                format!(" rx=\"{}\"", n("r"))
            } else {
                String::new()
            };
            format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{radius} fill=\"{fill}\"/>",
                n("x"),
                n("y"),
                n("w"),
                n("h")
            )
        }
        "circle" => format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{fill}\"/>",
            n("cx"),
            n("cy"),
            n("r")
        ),
        "ellipse" => format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{fill}\"/>",
            n("cx"),
            n("cy"),
            n("rx"),
            n("ry")
        ),
        "poly" => format!(
            "<polygon points=\"{}\" fill=\"{fill}\"/>",
            points(&p["pts"])
        ),
        "line" => format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{fill}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
            points(&p["pts"]),
            default_number(&p["w"], CUT)
        ),
        _ => String::new(),
    }
}
fn layers(shape: &Value) -> (String, String) {
    (
        list(&shape["body"])
            .iter()
            .map(|p| primitive(p, "currentColor"))
            .collect(),
        list(&shape["cuts"])
            .iter()
            .map(|p| primitive(p, "#000"))
            .collect(),
    )
}
fn markup(shape: &Value, id: &str) -> String {
    let [x, y, w, h] = bounds(shape);
    let (body, cuts) = layers(shape);
    if cuts.is_empty() {
        return format!("<g>{body}</g>");
    }
    format!(
        "<mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#fff\"/>{cuts}</mask><g mask=\"url(#{id})\">{body}</g>",
        x - 2.0,
        y - 2.0,
        w + 4.0,
        h + 4.0,
        x - 2.0,
        y - 2.0,
        w + 4.0,
        h + 4.0
    )
}
fn svg(shape: &Value, id: &str, pad: f64) -> String {
    let [x, y, w, h] = bounds(shape);
    let viewbox = [x - pad, y - pad, w + 2.0 * pad, h + 2.0 * pad]
        .map(|v| round(v).to_string())
        .join(" ");
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{viewbox}\" fill=\"currentColor\">{}</svg>\n",
        markup(shape, &format!("cut-{id}"))
    )
}
fn compose(weapon: &str, parts: &Value) -> Result<Value, String> {
    let art = &catalog()["weapons"][weapon];
    if art.is_null() {
        return Err(format!("No art for weapon {weapon}"));
    }
    let mut shapes = Vec::new();
    // Entry arrays preserve the caller's painter order across the JSON boundary.
    for entry in list(parts) {
        let slot = string(&entry[0]);
        let id = string(&entry[1]);
        let part = &catalog()["parts"][id];
        if part.is_null() {
            return Err(format!("No art for {id}"));
        }
        let anchor = &art["anchors"][slot];
        if anchor.is_null() {
            return Err(format!("No anchor for {slot}"));
        }
        let scale = if part["units"] == "rifle" && art["units"] == "pistol" {
            2.5
        } else {
            1.0
        };
        shapes.push(place(
            &part["mount"],
            number(&anchor[0]),
            number(&anchor[1]),
            scale,
        ));
    }
    shapes.push(art["body"].clone());
    Ok(merge(&shapes))
}
fn item(id: &str, stock: &Value) -> Result<Value, String> {
    let wear = &catalog()["wear"][id];
    if !wear.is_null() {
        return Ok(wear.clone());
    }
    let part = &catalog()["parts"][id];
    if !part.is_null() {
        return Ok(if !part["icon"].is_null() {
            part["icon"].clone()
        } else if !list(&part["mount"]["body"]).is_empty() {
            part["mount"].clone()
        } else {
            Value::Null
        });
    }
    for weapon in ["rifle", "shotgun", "sniper", "pistol"] {
        if id.starts_with(&format!("{weapon}-")) && !stock.is_null() {
            let entries = list(stock)
                .iter()
                .find(|v| v[0] == weapon)
                .map(|v| &v[1])
                .unwrap_or(&Value::Null);
            return compose(weapon, entries);
        }
    }
    Ok(Value::Null)
}
pub fn dispatch(method: &str, args: Value) -> Result<Value, String> {
    Ok(match method {
        "icons.catalog" => catalog().clone(),
        "icons.arc" => arc(&args),
        "icons.place" => place(
            &args[0],
            number(&args[1]),
            number(&args[2]),
            default_number(&args[3], 1.0),
        ),
        "icons.merge" => merge(list(&args)),
        "icons.bbox" => json!(bounds(&args[0])),
        "icons.layers" => {
            let (body, cuts) = layers(&args[0]);
            json!({"body":body,"cuts":cuts})
        }
        "icons.markup" => json!(markup(&args[0], string(&args[1]))),
        "icons.svg" => json!(svg(
            &args[0],
            string(&args[1]),
            default_number(&args[2], 4.0)
        )),
        "icons.composeWeapon" => compose(string(&args[0]), &args[1])?,
        "icons.itemArt" => item(string(&args[0]), &args[1])?,
        _ => return Err(format!("Unknown icons method {method}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transforms_geometry_and_stroke_together() {
        let shape = json!({"body":[{"k":"rect","x":-2,"y":3,"w":10,"h":4}],"cuts":[{"k":"line","pts":[0,1,2,3]}]});
        let transformed = place(&shape, 10.0, 20.0, 2.5);
        assert_eq!(bounds(&transformed), [5.0, 27.5, 25.0, 10.0]);
        assert_eq!(transformed["cuts"][0]["w"], 4.0);
    }
    #[test]
    fn empty_factory_mounts_have_no_inventory_icon() {
        for (id, part) in catalog()["parts"].as_object().unwrap() {
            if part["icon"].is_null() && list(&part["mount"]["body"]).is_empty() {
                assert_eq!(item(id, &Value::Null).unwrap(), Value::Null);
            }
        }
    }
    #[test]
    fn attachments_use_units_and_painter_order() {
        let parts = json!([
            ["optic", "rifle-optic-stock"],
            ["muzzle", "rifle-muzzle-stock"]
        ]);
        let composed = compose("pistol", &parts).unwrap();
        assert_eq!(composed["body"][0]["x"], 35.0);
        assert_eq!(
            composed["body"].as_array().unwrap().last(),
            catalog()["weapons"]["pistol"]["body"]["body"]
                .as_array()
                .unwrap()
                .last()
        );
    }
}
