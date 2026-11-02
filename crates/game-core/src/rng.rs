use serde_json::Value;
pub type State = [u32; 4];
pub fn state(v: &Value) -> State {
    std::array::from_fn(|i| v[i].as_u64().unwrap_or(0) as u32)
}
pub fn seed_value(v: &Value) -> State {
    seed(
        &v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| v.to_string()),
    )
}
pub fn seed(text: &str) -> State {
    let (mut h1, mut h2, mut h3, mut h4) =
        (1779033703u32, 3144134277u32, 1013904242u32, 2773480762u32);
    // JS strings use UTF-16 code units, including surrogate pairs.
    for k in text.encode_utf16().map(u32::from) {
        h1 = h2 ^ (h1 ^ k).wrapping_mul(597399067);
        h2 = h3 ^ (h2 ^ k).wrapping_mul(2869860233);
        h3 = h4 ^ (h3 ^ k).wrapping_mul(951274213);
        h4 = h1 ^ (h4 ^ k).wrapping_mul(2716044179);
    }
    h1 = (h3 ^ (h1 >> 18)).wrapping_mul(597399067);
    h2 = (h4 ^ (h2 >> 22)).wrapping_mul(2869860233);
    h3 = (h1 ^ (h3 >> 17)).wrapping_mul(951274213);
    h4 = (h2 ^ (h4 >> 19)).wrapping_mul(2716044179);
    [h1 ^ h2 ^ h3 ^ h4, h2 ^ h1, h3 ^ h1, h4 ^ h1]
}
pub fn next(s: &mut State) -> f64 {
    let [a, b, c, d] = *s;
    let d = d.wrapping_add(1);
    let t = a.wrapping_add(b).wrapping_add(d);
    *s = [
        b ^ (b >> 9),
        c.wrapping_add(c << 3),
        c.rotate_left(21).wrapping_add(t),
        d,
    ];
    f64::from(t) / 4294967296.0
}
pub fn roll(s: &mut State, p: f64) -> bool {
    next(s) < p
}
