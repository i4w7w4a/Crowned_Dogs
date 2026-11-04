//! The creation bay. The persona-bay picture is projected back from the camera that took it onto
//! a sheet laid over the hall's floor, walls and ceiling, so the creation camera can really move
//! through it: seen from the projector it is the picture itself, closer in the hall grows and
//! tilts like a room. A podium stands over the floor ring; cold rim light, a light shaft, glowing lamps and
//! slow dust keep the bay alive. A second, nearer shot of the bay is projected onto the same sheet
//! from its own camera, fitted to the first picture, and takes over as the camera closes on the
//! face: the hall gains detail in place instead of one picture giving way to another.
use super::view_utils::*;
use js_sys::{Array, Float32Array, Uint32Array};
use wasm_bindgen::prelude::*;

/// Where the picture was taken from: 0.74 m over the floor, 3.83 m in front of the ring's
/// centre, tilted 2.9° up, focal length 1460 px on the 1024×1536 picture. Measured from its
/// vanishing point and the ellipse of the floor ring, taken as 1.68 m across. The whole hall
/// stands `BACK` nearer than that, which sets the podium a little back on the painted ring.
pub const EYE: [f64; 3] = [0.0, 0.74, 3.83 + BACK];
const BACK: f64 = 0.08;
pub const TILT: f64 = 2.9;
const FOCAL: f64 = 1460.0;
const PICTURE: [f64; 2] = [1024.0, 1536.0];
/// The podium stands inside the painted ring, narrow enough for a phone's frame; its top is the
/// stand's platform height.
const PODIUM: f64 = 0.66;
/// Picture-plane tangents the body view shows above the sheet, from the projector's axis:
/// the boots with a little floor below, the crown with air above.
pub const BODY_TOP: f64 = 0.262;
pub const BODY_BOTTOM: f64 = -0.31;

/// The hall as the sheet takes it, in metres, floor at 0, the ring's centre at the origin: floor,
/// ceiling, side walls and one wall across at the face of the two machine columns. The bay
/// behind the columns lies flat on that wall: the face view stands far from both projectors,
/// and a sheet that dropped from a column to the back wall smeared the picture along its edge.
const HALL: [([f64; 3], [f64; 3]); 5] = [
    ([-7.0, -0.05, -8.5], [7.0, 0.0, 6.0]),
    ([-7.0, 5.7, -8.5], [7.0, 5.75, 6.0]),
    ([-7.0, 0.0, -4.15], [7.0, 9.5, -4.05]),
    ([-2.65, 0.0, -4.05], [-2.6, 5.7, 6.0]),
    ([2.6, 0.0, -4.05], [2.65, 5.7, 6.0]),
];
/// Lamps of the picture, as (pixel x, pixel y, the wall they hang on: axis and place, glow
/// size in metres, colour): the top bar, the columns' tubes, the far wall lights, the tanks.
const LAMPS: [(f64, f64, usize, f64, f64, u32); 13] = [
    (512.0, 248.0, 2, -4.05, 1.5, 0xe8f2ff),
    (283.0, 330.0, 2, -4.05, 0.9, 0xe6f0ff),
    (745.0, 330.0, 2, -4.05, 0.9, 0xe6f0ff),
    (282.0, 465.0, 2, -4.05, 0.45, 0x9fe9ff),
    (742.0, 465.0, 2, -4.05, 0.45, 0x9fe9ff),
    (282.0, 548.0, 2, -4.05, 0.45, 0x9fe9ff),
    (742.0, 548.0, 2, -4.05, 0.45, 0x9fe9ff),
    (282.0, 860.0, 2, -4.05, 0.4, 0x9fe9ff),
    (742.0, 860.0, 2, -4.05, 0.4, 0x9fe9ff),
    (45.0, 275.0, 0, -2.6, 1.0, 0xeef4ff),
    (978.0, 275.0, 0, 2.6, 1.0, 0xeef4ff),
    (90.0, 690.0, 0, -2.6, 0.8, 0x8fe6ff),
    (935.0, 690.0, 0, 2.6, 0.8, 0x8fe6ff),
];
/// The «Шлюз» bay, the empty turbine pit. Its picture is taken from the same camera over the same
/// ring (moved 32 px down onto it). The round wall of wicket-gate blades meets the floor 13.8 m
/// from the eye straight ahead and comes to about 10 m at the picture's edges: a wall across
/// with two side walls. The work lamps stand 9.2 m away, 2.7 m to each side.
const PIT: [([f64; 3], [f64; 3]); 5] = [
    ([-9.0, -0.05, -12.0], [9.0, 0.0, 6.0]),
    ([-9.0, 9.5, -12.0], [9.0, 9.55, 6.0]),
    ([-9.0, 0.0, -10.0], [9.0, 12.0, -9.9]),
    ([-4.05, 0.0, -9.9], [-4.0, 9.5, 6.0]),
    ([4.0, 0.0, -9.9], [4.05, 9.5, 6.0]),
];
const PIT_LAMPS: [(f64, f64, usize, f64, f64, u32); 8] = [
    (92.0, 567.0, 2, -5.35, 1.1, 0xeef4ff),
    (949.0, 567.0, 2, -5.35, 1.1, 0xeef4ff),
    (141.0, 403.0, 2, -9.9, 0.8, 0xe8f0ff),
    (357.0, 440.0, 2, -9.9, 0.8, 0xe8f0ff),
    (560.0, 450.0, 2, -9.9, 0.8, 0xe8f0ff),
    (758.0, 429.0, 2, -9.9, 0.8, 0xe8f0ff),
    (282.0, 211.0, 2, -9.9, 1.0, 0xe8f0ff),
    (833.0, 204.0, 2, -9.9, 1.0, 0xe8f0ff),
];
/// A base's bay: its pictures under `img/` (without the tier and the extension), the hall the
/// wide one lies on, and its lamps. Every bay is taken from `EYE` over the same floor ring.
pub struct Bay {
    pub photo: &'static str,
    /// The nearer shot for the face; without one the wide picture stays as the camera closes in.
    pub close: Option<&'static str>,
    /// Rendered stills of the default Persona on this stand (`<still>-male.webp`, `-female.webp`),
    /// shown while the 3D figure is still downloading on a slow connection.
    pub still: Option<&'static str>,
    hall: &'static [([f64; 3], [f64; 3])],
    lamps: &'static [(f64, f64, usize, f64, f64, u32)],
}
pub const SALT: Bay = Bay {
    photo: "base/persona-bay",
    close: Some("base/persona-bay-close"),
    still: Some("base/persona-bay"),
    hall: &HALL,
    lamps: &LAMPS,
};
pub const SLUICE: Bay = Bay {
    photo: "base-kaskad/persona-bay",
    close: None,
    still: None,
    hall: &PIT,
    lamps: &PIT_LAMPS,
};
/// Where the close shot was taken from, fitted to the first picture through the hall's boxes
/// (SIFT matches and the floor ring): 0.94 m up, 2.66 m in front of the ring, level within a
/// fifth of a degree, focal length 1633 px on the same 1024×1536 frame.
const CLOSE_EYE: [f64; 3] = [0.0, 0.937, 2.657 + BACK];
const CLOSE_TILT: f64 = 0.122;
const CLOSE_ROLL: f64 = 0.119;
const CLOSE_FOCAL: f64 = 1633.0;

const PROJECT_VERTEX: &str = "uniform mat4 cdProjector;
uniform mat4 cdNearProjector;
varying vec4 cdClip;
varying vec4 cdNearClip;
void main() {
  vec4 world = modelMatrix * vec4( position, 1.0 );
  cdClip = cdProjector * world;
  cdNearClip = cdNearProjector * world;
  gl_Position = projectionMatrix * viewMatrix * world;
}";
const PROJECT_FRAGMENT: &str = "uniform sampler2D cdPhoto;
uniform sampler2D cdClose;
uniform float cdBlur;
uniform float cdShade;
uniform float cdTime;
uniform float cdMix;
varying vec4 cdClip;
varying vec4 cdNearClip;
vec3 cdTap( sampler2D photo, vec2 uv, float bias ) {
  // Past its edges a picture mirrors back into itself.
  vec2 at = 1.0 - abs( 1.0 - mod( uv, 2.0 ) );
  return texture2D( photo, at, bias ).rgb;
}
vec3 cdLook( sampler2D photo, vec2 uv, float spread ) {
  vec3 c = cdTap( photo, uv, 0.0 );
  if ( cdBlur > 0.01 ) {
    // A little out of focus behind a close-up: a ring of taps, wider as the lens opens. The
    // centre keeps the picture's full resolution, so the hall stays crisp, only softer; the
    // ring reads a softer level, or every highlight would turn into a ring of dots.
    float r = cdBlur * spread;
    float soft = cdBlur * 5.0;
    c *= 0.36;
    for ( int i = 0; i < 8; i++ ) {
      float a = float( i ) * 0.7854 + 0.39;
      c += cdTap( photo, uv + vec2( cos( a ), sin( a ) * 0.667 ) * r, soft ) * 0.08;
    }
  }
  return c;
}
void main() {
  vec2 uv = cdClip.xy / max( cdClip.w, 1e-4 ) * 0.5 + 0.5;
  vec3 c = cdLook( cdPhoto, uv, 0.0035 );
  if ( cdMix > 0.0 ) {
    // The close shot through its own frame, its tone matched to the first picture; its pixels
    // lie 1.27 times denser on the walls, so the same blur reaches as many of them further.
    vec2 near = cdNearClip.xy / max( cdNearClip.w, 1e-4 ) * 0.5 + 0.5;
    float inside = smoothstep( 0.0, 0.05, near.x ) * smoothstep( 0.0, 0.05, 1.0 - near.x )
      * smoothstep( 0.0, 0.04, near.y ) * smoothstep( 0.0, 0.04, 1.0 - near.y ) * step( 0.0, cdNearClip.w );
    vec3 close = cdLook( cdClose, near, 0.0035 * 1.27 ) * vec3( 1.0, 1.04, 1.054 );
    c = mix( c, close, cdMix * inside );
  }
  // The bay's lamps breathe a little and now and then flicker.
  float hot = smoothstep( 0.42, 0.9, dot( c, vec3( 0.3, 0.6, 0.1 ) ) );
  float flicker = step( 0.985, fract( cdTime * 0.37 ) ) * sin( cdTime * 90.0 );
  c *= 1.0 + hot * ( 0.1 * sin( cdTime * 1.3 + uv.x * 5.0 ) + 0.12 * flicker );
  float outside = max( max( -uv.x, uv.x - 1.0 ), max( -uv.y, uv.y - 1.0 ) );
  c *= ( 1.0 - 0.85 * smoothstep( 0.0, 0.18, outside ) ) * cdShade * step( 0.0, cdClip.w );
  gl_FragColor = vec4( c, 1.0 );
  #include <colorspace_fragment>
}";
const DUST_VERTEX: &str = "uniform float cdTime;
uniform float cdScale;
attribute vec4 cdSeed;
varying float cdAlpha;
void main() {
  vec3 p = position;
  p.y = 0.15 + mod( p.y - cdTime * ( 0.012 + 0.03 * cdSeed.y ), 4.2 );
  p.x += sin( cdTime * 0.21 * ( 0.5 + cdSeed.y ) + cdSeed.x ) * 0.16;
  p.z += cos( cdTime * 0.17 * ( 0.5 + cdSeed.y ) + cdSeed.x * 1.7 ) * 0.16;
  vec4 mv = modelViewMatrix * vec4( p, 1.0 );
  gl_Position = projectionMatrix * mv;
  gl_PointSize = cdScale * cdSeed.z / max( -mv.z, 0.2 );
  // Brightest in the shaft over the podium, fading near the floor and the ceiling.
  float shaft = 1.0 - smoothstep( 0.35, 1.5, length( p.xz ) );
  cdAlpha = cdSeed.w * ( 0.18 + 0.82 * shaft ) * smoothstep( 0.15, 0.7, p.y ) * ( 1.0 - smoothstep( 3.3, 4.3, p.y ) );
  cdAlpha *= 0.55 + 0.45 * sin( cdTime * ( 0.6 + cdSeed.y ) + cdSeed.x * 3.0 );
}";
const DUST_FRAGMENT: &str = "varying float cdAlpha;
void main() {
  vec2 q = gl_PointCoord - 0.5;
  float a = cdAlpha * exp( -dot( q, q ) * 18.0 );
  gl_FragColor = vec4( vec3( 0.86, 0.92, 1.0 ) * a, 1.0 );
}";
const SHAFT_VERTEX: &str = "varying vec3 cdN;
varying vec3 cdV;
varying vec3 cdW;
varying float cdH;
void main() {
  vec4 w = modelMatrix * vec4( position, 1.0 );
  cdW = w.xyz;
  cdN = normalize( mat3( modelMatrix ) * normal );
  cdV = normalize( cameraPosition - w.xyz );
  cdH = uv.y;
  gl_Position = projectionMatrix * viewMatrix * w;
}";
const SHAFT_FRAGMENT: &str = "uniform float cdTime;
uniform float cdStrength;
varying vec3 cdN;
varying vec3 cdV;
varying vec3 cdW;
varying float cdH;
void main() {
  // Soft at the cone's silhouette, strongest toward the lamp, slowly stirring.
  float facing = pow( abs( dot( normalize( cdN ), normalize( cdV ) ) ), 2.2 );
  float fall = smoothstep( 0.0, 0.3, cdH ) * ( 0.3 + 0.7 * cdH );
  float stir = 0.72 + 0.28 * sin( cdW.y * 2.1 - cdTime * 0.35 + sin( atan( cdW.z, cdW.x ) * 3.0 + cdTime * 0.2 ) );
  float a = facing * fall * stir * cdStrength;
  gl_FragColor = vec4( vec3( 0.78, 0.88, 0.97 ) * a, 1.0 );
}";
const GLOW_FRAGMENT: &str = "uniform float cdTime;
uniform vec3 cdColor;
uniform float cdInner;
uniform float cdOuter;
uniform float cdStrength;
varying vec2 cdUv;
void main() {
  float r = length( cdUv - 0.5 ) * 2.0;
  float a = exp( -max( r - cdInner, 0.0 ) / ( cdOuter - cdInner ) * 4.0 ) * step( cdInner, r ) * ( 1.0 - smoothstep( 0.9, 1.0, r ) );
  a *= cdStrength * ( 0.9 + 0.1 * sin( cdTime * 1.1 ) );
  gl_FragColor = vec4( cdColor * a, 1.0 );
}";
const SHADOW_FRAGMENT: &str = "uniform float cdInner;
varying vec2 cdUv;
void main() {
  float r = length( cdUv - 0.5 ) * 2.0;
  float a = 0.62 * ( 1.0 - smoothstep( cdInner, 1.0, r ) );
  gl_FragColor = vec4( 0.0, 0.0, 0.0, a );
}";
const LAMP_VERTEX: &str = "uniform float cdSize;
varying vec2 cdUv;
void main() {
  cdUv = uv;
  vec4 mv = modelViewMatrix * vec4( 0.0, 0.0, 0.0, 1.0 );
  mv.xy += position.xy * cdSize;
  gl_Position = projectionMatrix * mv;
}";
const LAMP_FRAGMENT: &str = "uniform vec3 cdColor;
uniform float cdStrength;
uniform float cdTime;
uniform float cdPhase;
varying vec2 cdUv;
void main() {
  // A soft bloom round a lamp of the picture, breathing with the hall and now and then flickering.
  float r = length( cdUv - 0.5 ) * 2.0;
  float a = ( exp( -r * r * 9.0 ) * 0.7 + exp( -r * 3.2 ) * 0.3 ) * ( 1.0 - smoothstep( 0.75, 1.0, r ) );
  float flicker = step( 0.985, fract( cdTime * 0.37 + cdPhase * 0.13 ) ) * sin( cdTime * 90.0 );
  a *= cdStrength * ( 0.85 + 0.15 * sin( cdTime * 1.3 + cdPhase ) + 0.2 * flicker );
  gl_FragColor = vec4( cdColor * a, 1.0 );
}";
const UV_VERTEX: &str = "varying vec2 cdUv;
void main() {
  cdUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
}";

/// A projector: the camera that took a picture, as the matrix from the world to its frame.
fn projector(h: &JsValue, eye: [f64; 3], tilt: f64, roll: f64, focal: f64) -> JsValue {
    let fov = 2.0 * (PICTURE[1] / 2.0 / focal).atan().to_degrees();
    let camera = make(
        h,
        "THREE.PerspectiveCamera",
        &[num(fov), num(PICTURE[0] / PICTURE[1]), num(0.05), num(60.0)],
    );
    call(&get(&camera, "position"), "fromArray", &[floats(&eye)]);
    set(&camera, "rotation.x", &num(tilt.to_radians()));
    set(&camera, "rotation.z", &num(roll.to_radians()));
    call(&camera, "updateMatrixWorld", &[flag(true)]);
    let matrix = make(h, "THREE.Matrix4", &[]);
    call(
        &matrix,
        "multiplyMatrices",
        &[get(&camera, "projectionMatrix"), get(&camera, "matrixWorldInverse")],
    );
    matrix
}
/// Where a pixel of the picture lands on the wall `axis` = `place` (axis 0 for x, 2 for z).
fn on_picture(px: f64, py: f64, axis: usize, place: f64) -> [f64; 3] {
    let t = TILT.to_radians();
    let (x, y) = ((px - PICTURE[0] / 2.0) / FOCAL, (PICTURE[1] / 2.0 - py) / FOCAL);
    let dir = [x, y * t.cos() + t.sin(), y * t.sin() - t.cos()];
    let k = (place - EYE[axis]) / dir[axis];
    [0, 1, 2].map(|i| EYE[i] + dir[i] * k)
}
/// The camera's orientation when it stands where the picture was taken.
pub fn eye_rotation(h: &JsValue) -> JsValue {
    let q = make(h, "THREE.Quaternion", &[]);
    call(
        &q,
        "setFromEuler",
        &[make(h, "THREE.Euler", &[num(TILT.to_radians()), num(0.0), num(0.0)])],
    );
    q
}
/// How far along `dir` from `eye` the ray first enters the box, if it does.
fn enter(eye: [f64; 3], dir: [f64; 3], lo: [f64; 3], hi: [f64; 3]) -> Option<f64> {
    let (mut near, mut far) = (f64::NEG_INFINITY, f64::INFINITY);
    for i in 0..3 {
        if dir[i].abs() < 1e-12 {
            if eye[i] < lo[i] || eye[i] > hi[i] {
                return None;
            }
            continue;
        }
        let (a, b) = ((lo[i] - eye[i]) / dir[i], (hi[i] - eye[i]) / dir[i]);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && near > 1e-6).then_some(near)
}
/// The hall as one sheet seen from where the picture was taken: each vertex lies where its ray
/// through the picture first meets the hall's boxes. The boxes are flat walls only, so the sheet
/// never tears nor stretches. It reaches past the picture's edges, where the picture mirrors,
/// for wide frames and the close-up.
fn hall_sheet(hall: &[([f64; 3], [f64; 3])]) -> (Vec<f32>, Vec<u32>) {
    const COLUMNS: usize = 256;
    const ROWS: usize = 320;
    const ACROSS: [f64; 2] = [-0.4, 1.4];
    const DOWN: [f64; 2] = [-0.3, 1.3];
    let boxes: Vec<([f64; 3], [f64; 3])> = hall
        .iter()
        .map(|(lo, hi)| ([lo[0], lo[1], lo[2] + BACK], [hi[0], hi[1], hi[2] + BACK]))
        .collect();
    let t = TILT.to_radians();
    let mut positions = Vec::with_capacity((COLUMNS + 1) * (ROWS + 1) * 3);
    for row in 0..=ROWS {
        let py = (DOWN[0] + (DOWN[1] - DOWN[0]) * row as f64 / ROWS as f64) * PICTURE[1];
        for column in 0..=COLUMNS {
            let px = (ACROSS[0] + (ACROSS[1] - ACROSS[0]) * column as f64 / COLUMNS as f64) * PICTURE[0];
            let (x, y) = ((px - PICTURE[0] / 2.0) / FOCAL, (PICTURE[1] / 2.0 - py) / FOCAL);
            let dir = [x, y * t.cos() + t.sin(), y * t.sin() - t.cos()];
            let far = boxes
                .iter()
                .filter_map(|(lo, hi)| enter(EYE, dir, *lo, *hi))
                .fold(f64::INFINITY, f64::min);
            let far = if far.is_finite() { far } else { 20.0 };
            positions.extend((0..3).map(|i| (EYE[i] + dir[i] * far) as f32));
        }
    }
    let mut indices = Vec::with_capacity(COLUMNS * ROWS * 6);
    let at = |row: usize, column: usize| (row * (COLUMNS + 1) + column) as u32;
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let (a, b) = (at(row, column), at(row, column + 1));
            let (c, d) = (at(row + 1, column + 1), at(row + 1, column));
            indices.extend_from_slice(&[a, d, c, a, c, b]);
        }
    }
    (positions, indices)
}
fn uniforms(values: &[(&str, JsValue)]) -> JsValue {
    let out = object();
    for (key, value) in values {
        set(&out, key, &options(&[("value", value.clone())]));
    }
    out
}
fn shader(h: &JsValue, vertex: &str, fragment: &str, values: &[(&str, JsValue)], extra: &[(&str, JsValue)]) -> JsValue {
    let mut list = vec![
        ("vertexShader", text(vertex)),
        ("fragmentShader", text(fragment)),
        ("uniforms", uniforms(values)),
    ];
    list.extend_from_slice(extra);
    make(h, "THREE.ShaderMaterial", &[options(&list)])
}
fn additive(h: &JsValue) -> Vec<(&'static str, JsValue)> {
    vec![
        ("transparent", flag(true)),
        ("depthWrite", flag(false)),
        ("blending", get(h, "THREE.AdditiveBlending")),
        ("toneMapped", flag(false)),
    ]
}
fn texture(h: &JsValue, url: &str, color: bool, anisotropy: f64) -> JsValue {
    let t = call(&make(h, "THREE.TextureLoader", &[]), "load", &[text(url)]);
    if color {
        set(&t, "colorSpace", &get(h, "THREE.SRGBColorSpace"));
    }
    set(&t, "anisotropy", &num(anisotropy));
    t
}
/// A flat disc on the floor or the podium, for glows and the contact shadow.
fn disc(h: &JsValue, radius: f64, y: f64, material: JsValue) -> JsValue {
    let mesh = make(
        h,
        "THREE.Mesh",
        &[make(h, "THREE.PlaneGeometry", &[num(radius * 2.0), num(radius * 2.0)]), material],
    );
    set(&mesh, "rotation.x", &num(-std::f64::consts::FRAC_PI_2));
    set(&mesh, "position.y", &num(y));
    mesh
}

/// Builds the bay once: the projected hall, lights, shaft and dust go in `bay` (fixed in the
/// scene), the podium in `podium` (it turns with the Persona).
pub fn build(s: &JsValue, anisotropy: f64) {
    let h = get(s, "host");
    let base = string(&h, "base");
    let top = number(&h, "PLATFORM_TOP");
    let bay = make(&h, "THREE.Group", &[]);
    set(&bay, "name", &text("Creation_Bay"));

    // The hall, one mesh, painted by both projectors. The face view magnifies the pictures about
    // three times, so every device but the weakest gets them doubled (Lanczos and a light
    // unsharp mask, from the 1024×1536 originals).
    let tier = if number(s, "quality.textures") >= 1024.0 { ".2048" } else { "" };
    let picture = |name: &str| {
        texture(&h, &format!("{base}img/{name}{tier}.webp"), true, anisotropy)
    };
    let look = super::base_region::current().bay;
    let photo = picture(look.photo);
    let close = look.close.map_or_else(|| photo.clone(), picture);
    let (positions, indices) = hall_sheet(look.hall);
    let geometry = make(&h, "THREE.BufferGeometry", &[]);
    call(
        &geometry,
        "setAttribute",
        &[
            text("position"),
            make(&h, "THREE.BufferAttribute", &[Float32Array::from(&positions[..]).into(), num(3.0)]),
        ],
    );
    call(
        &geometry,
        "setIndex",
        &[make(&h, "THREE.BufferAttribute", &[Uint32Array::from(&indices[..]).into(), num(1.0)])],
    );
    let hall = shader(
        &h,
        PROJECT_VERTEX,
        PROJECT_FRAGMENT,
        &[
            ("cdPhoto", photo),
            ("cdClose", close),
            ("cdProjector", projector(&h, EYE, TILT, 0.0, FOCAL)),
            ("cdNearProjector", projector(&h, CLOSE_EYE, CLOSE_TILT, CLOSE_ROLL, CLOSE_FOCAL)),
            ("cdBlur", num(0.0)),
            ("cdShade", num(1.0)),
            ("cdTime", num(0.0)),
            ("cdMix", num(0.0)),
        ],
        &[("side", get(&h, "THREE.DoubleSide")), ("toneMapped", flag(false))],
    );
    let mesh = make(&h, "THREE.Mesh", &[geometry, hall.clone()]);
    set(&mesh, "name", &text("Bay_Projection"));
    set(&mesh, "frustumCulled", &flag(false));
    call(&bay, "add", &[mesh]);

    // Grounding: a soft shadow round the podium's foot, its light spilling on the floor.
    let shadow = shader(
        &h,
        UV_VERTEX,
        SHADOW_FRAGMENT,
        &[("cdInner", num(PODIUM / 1.7))],
        &[("transparent", flag(true)), ("depthWrite", flag(false))],
    );
    call(&bay, "add", &[disc(&h, 1.7, 0.002, shadow)]);
    let spill = shader(
        &h,
        UV_VERTEX,
        GLOW_FRAGMENT,
        &[
            ("cdTime", num(0.0)),
            ("cdColor", make(&h, "THREE.Color", &[num(0.62), num(0.86), num(1.0)])),
            ("cdInner", num(PODIUM / 1.6)),
            ("cdOuter", num(1.0)),
            ("cdStrength", num(0.85)),
        ],
        &additive(&h),
    );
    call(&bay, "add", &[disc(&h, 1.6, 0.004, spill.clone())]);
    // The hall is painted, not lit, so the key light's shadows of the podium and the Persona
    // fall on a catcher laid over the floor.
    let catcher = make(
        &h,
        "THREE.ShadowMaterial",
        &[options(&[
            ("color", make(&h, "THREE.Color", &[num(0x05080a as f64)])),
            ("opacity", num(0.55)),
            ("depthWrite", flag(false)),
            ("polygonOffset", flag(true)),
            ("polygonOffsetFactor", num(-1.0)),
            ("polygonOffsetUnits", num(-4.0)),
        ])],
    );
    let floor = disc(&h, 3.0, 0.003, catcher);
    set(&floor, "name", &text("Bay_Shadows"));
    set(&floor, "receiveShadow", &flag(true));
    call(&bay, "add", &[floor]);

    // A light shaft from a lamp over the podium, out of the picture.
    let cone = make(
        &h,
        "THREE.CylinderGeometry",
        &[num(0.3), num(0.88), num(5.5), num(48.0), num(1.0), flag(true)],
    );
    let mut extra = additive(&h);
    extra.push(("side", get(&h, "THREE.DoubleSide")));
    let shaft = shader(
        &h,
        SHAFT_VERTEX,
        SHAFT_FRAGMENT,
        &[("cdTime", num(0.0)), ("cdStrength", num(0.15))],
        &extra,
    );
    let beam = make(&h, "THREE.Mesh", &[cone, shaft.clone()]);
    set(&beam, "position.y", &num(top + 2.75));
    set(&beam, "renderOrder", &num(3.0));
    call(&bay, "add", &[beam]);

    // Dust in the light.
    let count = 560;
    let mut points = Vec::with_capacity(count * 3);
    let mut seeds = Vec::with_capacity(count * 4);
    let mut seed = 0x2545_f491_u32;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32
    };
    for _ in 0..count {
        let (a, r) = (random() * std::f32::consts::TAU, random().sqrt() * 2.2);
        points.extend_from_slice(&[a.cos() * r, random() * 4.2, a.sin() * r * 0.9 - 0.2]);
        seeds.extend_from_slice(&[random() * 6.3, random(), 0.006 + random() * 0.012, 0.35 + random() * 0.65]);
    }
    let cloud = make(&h, "THREE.BufferGeometry", &[]);
    call(
        &cloud,
        "setAttribute",
        &[
            text("position"),
            make(&h, "THREE.BufferAttribute", &[Float32Array::from(&points[..]).into(), num(3.0)]),
        ],
    );
    call(
        &cloud,
        "setAttribute",
        &[
            text("cdSeed"),
            make(&h, "THREE.BufferAttribute", &[Float32Array::from(&seeds[..]).into(), num(4.0)]),
        ],
    );
    let dust = shader(
        &h,
        DUST_VERTEX,
        DUST_FRAGMENT,
        &[("cdTime", num(0.0)), ("cdScale", num(1000.0))],
        &additive(&h),
    );
    let motes = make(&h, "THREE.Points", &[cloud, dust.clone()]);
    set(&motes, "frustumCulled", &flag(false));
    set(&motes, "renderOrder", &num(4.0));
    call(&bay, "add", &[motes]);

    // Blooms on the picture's lamps: the light seems to hang in the air in front of them.
    let lamps = Array::new();
    let quad = make(&h, "THREE.PlaneGeometry", &[num(1.0), num(1.0)]);
    for (i, (px, py, axis, place, size, color)) in look.lamps.iter().enumerate() {
        let place = if *axis == 2 { place + BACK } else { *place };
        let at = on_picture(*px, *py, *axis, place);
        let glow = shader(
            &h,
            LAMP_VERTEX,
            LAMP_FRAGMENT,
            &[
                ("cdColor", make(&h, "THREE.Color", &[num(*color as f64)])),
                ("cdSize", num(*size)),
                ("cdStrength", num(0.0)),
                ("cdTime", num(0.0)),
                ("cdPhase", num(i as f64 * 1.7)),
            ],
            &additive(&h),
        );
        let mesh = make(&h, "THREE.Mesh", &[quad.clone(), glow.clone()]);
        // A little in front of its wall, so the wall never cuts it.
        let toward = [0, 1, 2].map(|k| at[k] + (EYE[k] - at[k]) * 0.04);
        call(&get(&mesh, "position"), "fromArray", &[floats(&toward)]);
        set(&mesh, "frustumCulled", &flag(false));
        set(&mesh, "renderOrder", &num(3.0));
        call(&bay, "add", &[mesh]);
        lamps.push(&glow);
    }

    // The bay's own light on the Persona: a cold top light from the shaft, rims from the
    // columns' tubes behind, a cyan lift from the podium's ring.
    let spot = make(
        &h,
        "THREE.SpotLight",
        &[num(0xdfe9f2 as f64), num(46.0), num(0.0), num(0.34), num(0.75), num(2.0)],
    );
    call(&get(&spot, "position"), "set", &[num(0.0), num(5.6), num(0.35)]);
    call(&get(&spot, "target.position"), "set", &[num(0.0), num(top + 0.9), num(0.0)]);
    call(&bay, "add", &[spot.clone(), get(&spot, "target")]);
    for (color, power, pos) in [
        (0xa9d4ec, 3.6, [-2.1, 2.6, -3.4]),
        (0xc9e2f0, 2.7, [2.3, 2.2, -3.1]),
    ] {
        let light = make(&h, "THREE.DirectionalLight", &[num(color as f64), num(power)]);
        call(&get(&light, "position"), "fromArray", &[floats(&pos)]);
        call(&get(&light, "target.position"), "set", &[num(0.0), num(top + 1.0), num(0.0)]);
        call(&bay, "add", &[light.clone(), get(&light, "target")]);
    }
    let lift = make(
        &h,
        "THREE.PointLight",
        &[num(0x8fdcf4 as f64), num(2.6), num(2.4), num(2.0)],
    );
    call(&get(&lift, "position"), "set", &[num(0.0), num(top + 0.12), num(0.95)]);
    call(&bay, "add", &[lift.clone()]);

    set(&bay, "visible", &flag(false));
    call(&get(s, "scene"), "add", &[bay.clone()]);
    set(s, "bay", &bay);
    set(
        s,
        "bayParts",
        &options(&[
            ("hall", hall),
            ("spill", spill),
            ("shaft", shaft),
            ("dust", dust),
            ("lift", lift),
            ("lamps", lamps.into()),
            ("close", flag(look.close.is_some())),
        ]),
    );
    build_podium(s, &h, &base, top, anisotropy);
}

/// A machined steel plinth over the floor ring: a bolted, segmented top with a tread lip, a
/// chamfered skirt and a light strip glowing in a groove round its foot.
fn build_podium(s: &JsValue, h: &JsValue, base: &str, top: f64, anisotropy: f64) {
    let podium = make(h, "THREE.Group", &[]);
    set(&podium, "name", &text("Creation_Podium"));
    let face = make(
        h,
        "THREE.MeshStandardMaterial",
        &[options(&[
            ("color", make(h, "THREE.Color", &[num(1.0), num(1.0), num(1.0)])),
            ("map", texture(h, &format!("{base}img/base/podium-top.webp"), true, anisotropy)),
            ("normalMap", texture(h, &format!("{base}img/base/podium-top-n.webp"), false, anisotropy)),
            ("roughnessMap", texture(h, &format!("{base}img/base/podium-top-orm.webp"), false, anisotropy)),
            ("metalness", num(1.0)),
            ("roughness", num(1.0)),
            ("envMapIntensity", num(0.9)),
        ])],
    );
    set(&face, "metalnessMap", &get(&face, "roughnessMap"));
    set(&face, "aoMap", &get(&face, "roughnessMap"));
    call(&get(&face, "normalScale"), "set", &[num(0.9), num(0.9)]);
    let plate = make(
        h,
        "THREE.Mesh",
        &[make(h, "THREE.CircleGeometry", &[num(PODIUM - 0.035), num(128.0)]), face],
    );
    set(&plate, "rotation.x", &num(-std::f64::consts::FRAC_PI_2));
    set(&plate, "position.y", &num(top));
    set(&plate, "receiveShadow", &flag(true));
    call(&podium, "add", &[plate]);

    // The skirt: chamfered top edge, a straight side, a recessed groove for the light, a foot.
    let r = PODIUM;
    let profile: [(f64, f64); 12] = [
        (r - 0.036, top),
        (r - 0.018, top - 0.004),
        (r - 0.004, top - 0.018),
        (r, top - 0.03),
        (r, top - 0.082),
        (r - 0.012, top - 0.088),
        (r - 0.012, top - 0.104),
        (r, top - 0.11),
        (r, top - 0.128),
        (r + 0.012, top - 0.134),
        (r + 0.012, 0.0),
        (r - 0.2, 0.0),
    ];
    let points = Array::new();
    for (x, y) in profile.iter().rev() {
        points.push(&make(h, "THREE.Vector2", &[num(*x), num(*y)]));
    }
    let skirt = make(
        h,
        "THREE.MeshStandardMaterial",
        &[options(&[
            ("color", num(0x3a3f44 as f64)),
            ("metalness", num(0.92)),
            ("roughness", num(0.36)),
            ("envMapIntensity", num(1.1)),
        ])],
    );
    let lathe = make(
        h,
        "THREE.Mesh",
        &[make(h, "THREE.LatheGeometry", &[points.into(), num(160.0)]), skirt],
    );
    set(&lathe, "castShadow", &flag(true));
    set(&lathe, "receiveShadow", &flag(true));
    call(&podium, "add", &[lathe]);

    // The light strip in the groove.
    let strip = make(h, "THREE.MeshBasicMaterial", &[options(&[("toneMapped", flag(false))])]);
    call(&get(&strip, "color"), "setRGB", &[num(1.25), num(1.75), num(2.0)]);
    let ring = make(
        h,
        "THREE.Mesh",
        &[
            make(h, "THREE.TorusGeometry", &[num(r - 0.008), num(0.0045), num(8.0), num(200.0)]),
            strip.clone(),
        ],
    );
    set(&ring, "rotation.x", &num(-std::f64::consts::FRAC_PI_2));
    set(&ring, "position.y", &num(top - 0.096));
    call(&podium, "add", &[ring]);
    // Its halo on the skirt, a soft band that stands in for bloom.
    let halo = shader(
        h,
        "varying float cdY;
void main() {
  cdY = position.y;
  gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
}",
        "uniform float cdCentre;
uniform float cdStrength;
varying float cdY;
void main() {
  float a = exp( -pow( ( cdY - cdCentre ) / 0.02, 2.0 ) ) * cdStrength;
  gl_FragColor = vec4( vec3( 0.55, 0.85, 1.0 ) * a, 1.0 );
}",
        &[("cdCentre", num(top - 0.096)), ("cdStrength", num(0.7))],
        &{
            let mut e = additive(h);
            e.push(("side", get(h, "THREE.DoubleSide")));
            e
        },
    );
    let band = make(
        h,
        "THREE.Mesh",
        &[
            make(
                h,
                "THREE.CylinderGeometry",
                &[num(r + 0.004), num(r + 0.004), num(0.08), num(160.0), num(1.0), flag(true)],
            ),
            halo,
        ],
    );
    set(&band, "position.y", &num(top - 0.096));
    set(&band, "renderOrder", &num(2.0));
    call(&podium, "add", &[band]);

    set(&podium, "visible", &flag(false));
    call(&get(s, "stage.rotating"), "add", &[podium.clone()]);
    set(s, "podium", &podium);
    set(s, "bayParts.strip", &strip);
}

/// Shows or hides the bay with creation mode.
pub fn show(s: &JsValue, on: bool) {
    for name in ["bay", "podium"] {
        let part = get(s, name);
        if exists(&part) {
            set(&part, "visible", &flag(on));
        }
    }
}

/// Brings the close shot in as the camera closes on the face (`k` 0 body, 1 face), mostly while
/// the camera moves fastest.
pub fn close_in(s: &JsValue, k: f64) {
    let hall = get(s, "bayParts.hall");
    if !exists(&hall) || !yes(s, "bayParts.close") {
        return;
    }
    let x = ((k - 0.15) / 0.7).clamp(0.0, 1.0);
    set(&get(&hall, "uniforms.cdMix"), "value", &num(x * x * (3.0 - 2.0 * x)));
}

/// Moves the bay on: time for the shaders, the lens's blur and dimming as the camera closes on
/// the face (`focus` 0 body, 1 face), the dust's scale for the drawing buffer.
pub fn tick(s: &JsValue, time: f64, focus: f64, buffer_height: f64) {
    let parts = get(s, "bayParts");
    if !exists(&parts) {
        return;
    }
    let t = num(time % 3600.0);
    for name in ["hall", "spill", "shaft", "dust"] {
        let u = get(&get(&parts, name), "uniforms");
        if exists(&get(&u, "cdTime")) {
            set(&get(&u, "cdTime"), "value", &t);
        }
    }
    let hall = get(&get(&parts, "hall"), "uniforms");
    set(&get(&hall, "cdBlur"), "value", &num(focus * 0.8));
    set(&get(&hall, "cdShade"), "value", &num(1.0 - 0.24 * focus));
    // The lamps' blooms give way to the close shot, whose lamps stand a little elsewhere.
    let close = number(&hall, "cdMix.value");
    for glow in Array::from(&get(&parts, "lamps")).iter() {
        let u = get(&glow, "uniforms");
        set(&get(&u, "cdTime"), "value", &t);
        set(&get(&u, "cdStrength"), "value", &num(0.55 * (1.0 - close)));
    }
    set(
        &get(&get(&get(&parts, "dust"), "uniforms"), "cdScale"),
        "value",
        &num(buffer_height * 0.9),
    );
    // The ring's light breathes slowly.
    let pulse = 0.92 + 0.08 * (time * 1.1).sin();
    set(&get(&parts, "lift"), "intensity", &num(2.6 * pulse));
    let strip = get(&parts, "strip");
    if exists(&strip) {
        call(
            &get(&strip, "color"),
            "setRGB",
            &[num(1.25 * pulse), num(1.75 * pulse), num(2.0 * pulse)],
        );
    }
}

