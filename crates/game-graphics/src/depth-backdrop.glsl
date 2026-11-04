uniform sampler2D uColor, uDepth;
uniform vec2 uCover, uShift;
// A talking portrait (Sedoy in the briefing): mouth and eye shapes cut from edits of the same
// photograph, 2 x 2 cells with their own soft alpha (slightly open, open, rounded, eyes shut).
uniform sampler2D uFace;
uniform vec4 uFaceMix;   // how much of each cell shows now
uniform vec4 uMouthRect; // where the mouth cells sit in the picture: u0, v0, u1, v1 (v from the top)
uniform vec4 uEyesRect;  // where the eye cell sits
uniform vec4 uHead;      // the head's centre u, v and radii: it sways a little, the chest breathes
varying vec2 vUv;

vec4 cdFaceCell(vec2 p, vec4 rect, vec2 cell) {
    vec2 q = (p - rect.xy) / (rect.zw - rect.xy);
    if (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0) return vec4(0.0);
    vec2 a = (cell + q) * 0.5;
    return texture2D(uFace, vec2(a.x, 1.0 - a.y));
}
vec3 cdFace(vec3 col, vec2 p) {
    vec4 e = cdFaceCell(p, uMouthRect, vec2(0.0, 0.0));
    vec4 a = cdFaceCell(p, uMouthRect, vec2(1.0, 0.0));
    vec4 o = cdFaceCell(p, uMouthRect, vec2(0.0, 1.0));
    vec4 shut = cdFaceCell(p, uEyesRect, vec2(1.0, 1.0));
    vec3 w = uFaceMix.xyz * vec3(e.a, a.a, o.a);
    col = col * (1.0 - min(w.x + w.y + w.z, 1.0)) + e.rgb * w.x + a.rgb * w.y + o.rgb * w.z;
    return mix(col, shut.rgb, uFaceMix.w * shut.a);
}
// The man breathes (his shoulders rise more than his head) and his head wanders a little and
// dips on the open syllables. `depth` tells him from the room behind.
vec2 cdFaceWarp(vec2 p, float depth, float t) {
    float man = smoothstep(0.45, 0.7, depth);
    float rise = 0.5 - 0.5 * cos(6.2832 * t / 4.6);
    vec2 d = vec2(0.0, man * rise * mix(0.0008, 0.0019, smoothstep(0.35, 0.7, p.y)));
    float head = smoothstep(1.0, 0.65, length((p - uHead.xy) / uHead.zw));
    vec2 wander = vec2(cdNoise1(t * 0.23) - 0.5, cdNoise1(t * 0.19 + 4.0) - 0.5) * 0.004;
    return d + head * (wander - vec2(0.0, 0.0012 * (uFaceMix.y + uFaceMix.z)));
}

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
    // On the live qualities the room lives (live-scene.glsl, above).
    vec2 p = vec2(uv.x, 1.0 - uv.y);
    vec2 life = cdLiveWarp(p, uTime);
    if (uHead.z > 0.0) life += cdFaceWarp(p, texture2D(uDepth, uv).r, uTime);
    uv = clamp(uv + vec2(life.x, -life.y) * uLive, vec2(0.0), vec2(1.0));
    p = vec2(uv.x, 1.0 - uv.y);
    vec4 color = texture2D(uColor, uv);
    if (uMouthRect.z > 0.0) color.rgb = mix(color.rgb, cdFace(color.rgb, p), uLive);
    vec3 alive = cdLiveLight(color.rgb, p, texture2D(uDepth, uv).r, uTime);
    gl_FragColor = vec4(mix(color.rgb, alive, uLive), color.a);
}
