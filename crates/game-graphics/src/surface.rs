//! Physical surface-profile validation shared by the browser and asset tools.
//! A corrupt or incompatible optional profile falls back to the original PBR.
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaterialProfile {
    eligible: bool,
    depth_meters: f64,
    midpoint: f64,
    #[serde(default)]
    detail_depth_meters: f64,
    #[serde(default)]
    fabric_depth_meters: f64,
    #[serde(default)]
    fabric_period_meters: f64,
    #[serde(default)]
    uv_set: u32,
    resolution: [u32; 2],
    relief: String,
}
impl MaterialProfile {
    fn valid(&self) -> bool {
        self.eligible
            && (0.000001..=0.008).contains(&self.depth_meters)
            && (0.4..=0.6).contains(&self.midpoint)
            && (0.0..=self.depth_meters).contains(&self.detail_depth_meters)
            && (0.0..=self.depth_meters).contains(&self.fabric_depth_meters)
            && (self.fabric_depth_meters == 0.0
                || (self.fabric_period_meters >= self.fabric_depth_meters * 2.0
                    && self.fabric_period_meters <= 0.02))
            && self.uv_set == 0
            && self.resolution.iter().all(|n| (32..=8192).contains(n))
            && self.relief.starts_with("tex/relief/")
            && !self.relief.contains("..")
    }
    fn in_model_units(&self, units_per_meter: f64) -> Value {
        json!({
            "depthUnits": self.depth_meters * units_per_meter,
            "detailDepthUnits": self.detail_depth_meters * units_per_meter,
            "fabricDepthUnits": self.fabric_depth_meters * units_per_meter,
            "fabricPeriodUnits": self.fabric_period_meters * units_per_meter,
            "midpoint": self.midpoint,
            "source": self.relief,
        })
    }
}

pub fn profile(args: &Value) -> Value {
    let Some(material) = args.get("material") else {
        return Value::Null;
    };
    let Ok(material) = serde_json::from_value::<MaterialProfile>(material.clone()) else {
        return Value::Null;
    };
    let units = args["unitsPerMeter"].as_f64().unwrap_or(1.0);
    if !material.valid() || !(0.0001..=10000.0).contains(&units) {
        return Value::Null;
    }
    material.in_model_units(units)
}

pub fn program(args: &Value) -> Result<Value, String> {
    let quality = args["quality"].as_str().unwrap_or("medium");
    let (steps, refine, shadows) = match quality {
        "minimal" | "low" => (0, 0, 0),
        // Medium is the phones' default: a coarser march and no self-shadow in the weave.
        "medium" => (12, 3, 0),
        "high" => (36, 5, 10),
        "ultra" => (56, 6, 14),
        _ => return Err(format!("Unknown graphics quality: {quality}")),
    };
    Ok(json!({
        "common": include_str!("surface-pom.glsl"),
        "steps": steps,
        "refine": refine,
        "shadowSteps": shadows,
        "version": "cd-surface-relief-v2",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn material() -> Value {
        json!({"eligible":true,"depthMeters":0.0024,"midpoint":128.0/255.0,
            "detailDepthMeters":0.0016,"fabricDepthMeters":0.0004,"fabricPeriodMeters":0.0018,
            "resolution":[2048,2048],"relief":"tex/relief/vest.relief.png","uvSet":0})
    }
    #[test]
    fn model_units_preserve_the_same_physical_surface() {
        let metre = profile(&json!({"material":material(),"unitsPerMeter":1.0}));
        let centimetre = profile(&json!({"material":material(),"unitsPerMeter":100.0}));
        for field in [
            "depthUnits",
            "detailDepthUnits",
            "fabricDepthUnits",
            "fabricPeriodUnits",
        ] {
            assert!(
                (centimetre[field].as_f64().unwrap() / 100. - metre[field].as_f64().unwrap()).abs()
                    < 1e-12
            );
        }
    }
    #[test]
    fn invalid_profiles_keep_original_materials() {
        for (key, value) in [
            ("depthMeters", json!(0.02)),
            ("fabricDepthMeters", json!(0.004)),
            ("fabricPeriodMeters", json!(0.0001)),
            ("eligible", json!(false)),
            ("uvSet", json!(1)),
            ("resolution", json!([0, 0])),
            ("relief", json!("tex/relief/../bad.png")),
        ] {
            let mut m = material();
            m[key] = value;
            assert!(profile(&json!({"material":m})).is_null(), "{key}");
        }
        assert!(profile(&json!({"material":material(),"unitsPerMeter":0.0})).is_null());
    }
}
