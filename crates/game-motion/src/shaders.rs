pub const DUST_VERTEX: &str = include_str!("../shaders/dust.vert");
pub const DUST_FRAGMENT: &str = include_str!("../shaders/dust.frag");
pub const SPECULAR_AA: &str = include_str!("../shaders/specular-aa.glsl");
pub fn shadow(taps: u64) -> String {
    let phi = if taps >= 12 {
        "0.0"
    } else {
        "interleavedGradientNoise( gl_FragCoord.xy ) * PI2"
    };
    format!(
        "float phi = {phi};\nshadow = 0.0;\nfor ( int i = 0; i < {taps}; i ++ ) shadow += texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( i, {taps}, phi ) * radius, shadowCoord.z ) );\nshadow /= {taps}.0;"
    )
}
