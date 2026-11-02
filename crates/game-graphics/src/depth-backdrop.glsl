uniform sampler2D uColor, uDepth;
uniform vec2 uCover, uShift;
varying vec2 vUv;
void main() {
    vec2 base = (vUv - 0.5) * uCover + 0.5;
    vec2 uv = base;
    // Preserve the approved background's small five-step depth drift.
    // This is a backdrop warp, deliberately distinct from the podium's silhouette POM.
    for (int i = 0; i < 5; i++) uv = base + uShift * texture2D(uDepth, uv).r;
    if (uLive <= 0.0) {
        gl_FragColor = texture2D(uColor, uv);
        return;
    }
    // On high and ultra the room lives (live-scene.glsl, above).
    vec2 life = cdLiveWarp(vec2(uv.x, 1.0 - uv.y), uTime) * uLive;
    uv = clamp(uv + vec2(life.x, -life.y), vec2(0.0), vec2(1.0));
    vec4 color = texture2D(uColor, uv);
    vec3 alive = cdLiveLight(color.rgb, vec2(uv.x, 1.0 - uv.y), texture2D(uDepth, uv).r, uTime);
    gl_FragColor = vec4(mix(color.rgb, alive, uLive), color.a);
}
