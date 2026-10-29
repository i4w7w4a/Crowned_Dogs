//! Fixed photographic projector and two physical camera paths through the base.
use super::{bridge::*, projected_stage::RustProjectedStage};
use js_sys::Promise;
use wasm_bindgen::{JsCast, prelude::*};

const ASPECT: f64 = 1882. / 3344.;
const FOV: f64 = 55.;
const HORIZON: f64 = 0.30;
fn n(v: f64) -> JsValue {
    v.into()
}
fn ray(u: f64, v: f64, d: f64) -> [f64; 3] {
    let f = 0.5 / (FOV.to_radians() * 0.5).tan();
    [(u - 0.5) * ASPECT * d / f, 2. + (HORIZON - v) * d / f, -d]
}
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}
fn path(target: &str, t: f64) -> ([f64; 3], [f64; 3]) {
    let t = t.clamp(0., 1.);
    let e = t * t * t * (t * (6. * t - 15.) + 10.);
    let (end, mut look) = if target == "battle" {
        ([-0.40, 1.40, -2.55], ray(0.235, 0.527, 4.25))
    } else {
        ([-0.9, 1.9, -6.8], ray(0.275, 0.36, 18.))
    };
    look[1] += if target == "battle" { 0.38 } else { 2.0 };
    let mut pos = mix([0., 2., 0.], end, e);
    if target != "battle" {
        pos[0] += 1.40 * (std::f64::consts::PI * e).sin();
    }
    (pos, mix([0., 2., -8.], look, e))
}

#[wasm_bindgen]
pub struct RustHomeStage {
    canvas: JsValue,
    renderer: JsValue,
    camera: JsValue,
    stage: RustProjectedStage,
    ready: Promise,
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
        let ready = stage.load(url);
        Ok(Self {
            canvas,
            renderer,
            camera,
            stage,
            ready,
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
            let full_w = (h * ASPECT).max(w);
            let full_h = (w / ASPECT).max(h);
            let f = 0.5 / (FOV.to_radians() * 0.5).tan();
            let near = 0.05;
            let x0 = (full_w - w) * 0.5 / full_w;
            let x1 = 1. - x0;
            let y0 = (full_h - h) * 0.5 / full_h;
            let y1 = 1. - y0;
            call(
                &get(&self.camera, "projectionMatrix"),
                "makePerspective",
                &[
                    n((x0 - 0.5) * ASPECT * near / f),
                    n((x1 - 0.5) * ASPECT * near / f),
                    n((HORIZON - y0) * near / f),
                    n((HORIZON - y1) * near / f),
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
        let (pos, look) = path(target, progress);
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
    fn routes_have_shared_origin_and_distinct_metric_destinations() {
        assert_eq!(path("battle", 0.).0, [0., 2., 0.]);
        assert_eq!(path("fitting", 0.).0, [0., 2., 0.]);
        assert_eq!(path("battle", 1.).0, [-0.4, 1.4, -2.55]);
        let tunnel = path("fitting", 1.).0;
        assert!((tunnel[2] + 6.8).abs() < 1e-10);
        for route in ["battle", "fitting"] {
            let mut z = 0.;
            for i in 0..101 {
                let p = path(route, i as f64 / 100.).0;
                assert!(p[2] <= z + 1e-10);
                z = p[2];
            }
        }
    }
}
