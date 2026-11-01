//! The base uses the camera calibration and sampled paths exported with its geometry.
use super::{bridge::*, projected_stage::RustProjectedStage};
use js_sys::Promise;
use wasm_bindgen::{JsCast, prelude::*};

const ASPECT: f64 = 1882. / 3344.;
const FOV: f64 = 55.;
fn n(v: f64) -> JsValue {
    v.into()
}
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

#[derive(Clone, Copy)]
struct Pose {
    position: [f64; 3],
    target: [f64; 3],
}
struct View {
    fov: f64,
    aspect: f64,
    height: f64,
    horizon: f64,
    battle: Vec<Pose>,
    fitting: Vec<Pose>,
}
impl View {
    fn from_manifest(manifest: &serde_json::Value) -> Self {
        let calibration = &manifest["calibration"];
        let number = |key: &str, fallback: f64| {
            calibration[key]
                .as_f64()
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        };
        let samples = |route: &str| -> Vec<Pose> {
            let Some(samples) = manifest["cameraPaths"][route].as_array() else {
                return vec![];
            };
            if samples.len() < 2 {
                return vec![];
            }
            let vector = |value: &serde_json::Value| -> Option<[f64; 3]> {
                let values = value.as_array()?;
                if values.len() != 3 {
                    return None;
                }
                let mut out = [0.; 3];
                for (i, value) in values.iter().enumerate() {
                    out[i] = value.as_f64().filter(|v| v.is_finite())?;
                }
                Some(out)
            };
            // Reject an incomplete route as a whole instead of joining non-adjacent samples.
            samples
                .iter()
                .map(|sample| {
                    let position = vector(&sample["position"])?;
                    let target = vector(&sample["target"])?;
                    if position
                        .iter()
                        .zip(target)
                        .all(|(a, b)| (a - b).abs() < 1e-9)
                    {
                        return None;
                    }
                    Some(Pose { position, target })
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default()
        };
        Self {
            fov: number("fov", FOV).clamp(1., 179.),
            aspect: number("aspect", ASPECT).max(0.001),
            height: number("camHeight", 2.),
            horizon: number("horizon", 0.30).clamp(0.001, 0.999),
            battle: samples("battle"),
            fitting: samples("fitting"),
        }
    }
    fn pose(&self, route: &str, progress: f64) -> Pose {
        let samples = match route {
            "battle" => &self.battle,
            "fitting" => &self.fitting,
            _ => {
                return self
                    .battle
                    .first()
                    .or(self.fitting.first())
                    .copied()
                    .unwrap_or_else(|| self.origin());
            }
        };
        if samples.is_empty() {
            // Older manifests remain usable without replaying an uncalibrated camera journey.
            return self.origin();
        }
        let at = progress.clamp(0., 1.) * (samples.len() - 1) as f64;
        let left = at.floor() as usize;
        let right = (left + 1).min(samples.len() - 1);
        let t = at - left as f64;
        // Blender samples already contain their easing. Interpolate once, in raw progress.
        Pose {
            position: mix(samples[left].position, samples[right].position, t),
            target: mix(samples[left].target, samples[right].target, t),
        }
    }
    fn origin(&self) -> Pose {
        Pose {
            position: [0., self.height, 0.],
            target: [0., self.height, -8.],
        }
    }
}

#[wasm_bindgen]
pub struct RustHomeStage {
    canvas: JsValue,
    renderer: JsValue,
    camera: JsValue,
    stage: RustProjectedStage,
    ready: Promise,
    view: Option<View>,
    width: f64,
    height: f64,
    last: String,
    disposed: bool,
}
#[wasm_bindgen]
impl RustHomeStage {
    #[wasm_bindgen(constructor)]
    pub fn new(host: JsValue, canvas: JsValue, url: String) -> Result<RustHomeStage, JsValue> {
        let three = get(&host, "THREE");
        let renderer = js_sys::Reflect::construct(
            get(&three, "WebGLRenderer").unchecked_ref::<js_sys::Function>(),
            &array(&[object(&[
                ("canvas", canvas.clone()),
                ("antialias", true.into()),
                ("alpha", false.into()),
                ("powerPreference", "high-performance".into()),
            ])]),
        )?;
        set(&renderer, "toneMapping", &get(&three, "NoToneMapping"));
        call(&renderer, "setClearColor", &[n(0x101213 as f64), n(1.)]);
        let camera = construct(
            &get(&three, "PerspectiveCamera"),
            &[n(FOV), n(ASPECT), n(0.05), n(100.)],
        );
        let stage = RustProjectedStage::new(host, renderer.clone());
        let ready = stage.load(url, None);
        Ok(Self {
            canvas,
            renderer,
            camera,
            stage,
            ready,
            view: None,
            width: 0.,
            height: 0.,
            last: String::new(),
            disposed: false,
        })
    }
    #[wasm_bindgen(getter)]
    pub fn ready(&self) -> Promise {
        self.ready.clone()
    }
    pub fn draw(&mut self, target: &str, progress: f64) -> Result<(), JsValue> {
        if self.disposed || !self.stage.ready() {
            return Ok(());
        }
        if self.view.is_none() {
            self.view = Some(View::from_manifest(&self.stage.manifest_data()));
        }
        let view = self.view.as_ref().unwrap();
        let w = num(&get(&self.canvas, "clientWidth"));
        let h = num(&get(&self.canvas, "clientHeight"));
        if w <= 0. || h <= 0. {
            return Ok(());
        }
        let progress = if progress.is_finite() {
            progress.clamp(0., 1.)
        } else {
            0.
        };
        let key = format!("{target}:{progress:.5}:{w}:{h}");
        if key == self.last {
            return Ok(());
        }
        self.last = key;
        if w != self.width || h != self.height {
            self.width = w;
            self.height = h;
            call(
                &self.renderer,
                "setPixelRatio",
                &[n(num(&get(&global(), "devicePixelRatio")).clamp(1., 1.5))],
            );
            call(&self.renderer, "setSize", &[n(w), n(h), false.into()]);
            // Object-fit cover, at fixed focal length throughout both dolly paths.
            let full_w = (h * view.aspect).max(w);
            let full_h = (w / view.aspect).max(h);
            let f = 0.5 / (view.fov.to_radians() * 0.5).tan();
            let near = 0.05;
            let x0 = (full_w - w) * 0.5 / full_w;
            let x1 = 1. - x0;
            let y0 = (full_h - h) * 0.5 / full_h;
            let y1 = 1. - y0;
            call(
                &get(&self.camera, "projectionMatrix"),
                "makePerspective",
                &[
                    n((x0 - 0.5) * view.aspect * near / f),
                    n((x1 - 0.5) * view.aspect * near / f),
                    n((view.horizon - y0) * near / f),
                    n((view.horizon - y1) * near / f),
                    n(near),
                    n(100.),
                ],
            );
            call(
                &get(&self.camera, "projectionMatrixInverse"),
                "copy",
                &[get(&self.camera, "projectionMatrix")],
            );
            call(&get(&self.camera, "projectionMatrixInverse"), "invert", &[]);
        }
        let Pose {
            position: pos,
            target: look,
        } = view.pose(target, progress);
        call(&get(&self.camera, "position"), "set", &pos.map(n));
        call(&self.camera, "lookAt", &look.map(n));
        call(
            &self.canvas,
            "setAttribute",
            &["data-home-progress".into(), format!("{progress:.4}").into()],
        );
        call(
            &self.canvas,
            "setAttribute",
            &["data-home-target".into(), target.into()],
        );
        call(
            &self.canvas,
            "setAttribute",
            &[
                "data-camera-position".into(),
                format!("{:.3},{:.3},{:.3}", pos[0], pos[1], pos[2]).into(),
            ],
        );
        self.stage.draw_to_screen(self.camera.clone())
    }
    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.stage.dispose();
        let _ = try_call(&self.renderer, "dispose", &[]);
    }
}
impl Drop for RustHomeStage {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_camera_samples_are_interpolated_without_a_second_ease() {
        let view = View::from_manifest(&serde_json::json!({
            "calibration": { "fov": 63, "aspect": 0.6, "camHeight": 1.75, "horizon": 0.4 },
            "cameraPaths": { "battle": [
                { "position": [0, 1.75, 0], "target": [0, 1.75, -8] },
                { "position": [0.4, 1.6, -1], "target": [0.1, 1.2, -5] },
                { "position": [-0.6, 1.4, -4], "target": [-0.2, 1.0, -6] }
            ] }
        }));
        assert_eq!(view.pose("base", 0.).position, [0., 1.75, 0.]);
        assert_eq!(view.pose("battle", 0.25).position, [0.2, 1.675, -0.5]);
        assert_eq!(view.pose("battle", 0.5).position, [0.4, 1.6, -1.]);
        assert_eq!(view.pose("battle", 1.).position, [-0.6, 1.4, -4.]);
        assert_eq!(view.pose("fitting", 1.).position, [0., 1.75, 0.]);
    }
    #[test]
    fn incomplete_path_keeps_the_calibrated_projector_view() {
        let view = View::from_manifest(&serde_json::json!({
            "calibration": { "camHeight": 1.65 },
            "cameraPaths": { "battle": [
                { "position": [0, 1.65, 0], "target": [0, 1.65, -8] },
                { "position": [0, 1.65], "target": [0, 1.65, -8] },
                { "position": [0, 1.65, -3], "target": [0, 1.65, -8] }
            ] }
        }));
        assert_eq!(view.pose("battle", 0.8).position, [0., 1.65, 0.]);
    }
}
