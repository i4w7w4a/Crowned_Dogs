uniform sampler2D uColor, uDepth;
uniform vec2 uCover, uShift;
varying vec2 vUv;
void main() {
    vec2 base = (vUv - 0.5) * uCover + 0.5;
    vec2 uv = base;
    // Preserve the approved background's small five-step depth drift.
    // This is a backdrop warp, deliberately distinct from the podium's silhouette POM.
    for (int i = 0; i < 5; i++) uv = base + uShift * texture2D(uDepth, uv).r;
    gl_FragColor = texture2D(uColor, uv);
}
