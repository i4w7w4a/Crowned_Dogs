uniform sampler2D uColor;
uniform sampler2D uDepth;
uniform vec2 uCover;
uniform vec2 uPivot;
uniform vec2 uDepthTexel;
uniform float uZoom;
uniform float uParallax;
varying vec2 vUv;

float softDepth(vec2 at) {
  vec2 delta = uDepthTexel * 2.0;
  float value = texture2D(uDepth, at).r * 0.5;
  value += texture2D(uDepth, at + vec2(delta.x, 0.0)).r * 0.125;
  value += texture2D(uDepth, at - vec2(delta.x, 0.0)).r * 0.125;
  value += texture2D(uDepth, at + vec2(0.0, delta.y)).r * 0.125;
  value += texture2D(uDepth, at - vec2(0.0, delta.y)).r * 0.125;
  return clamp(value, 0.0, 1.0);
}
void main() {
  vec2 original = (vUv - 0.5) * uCover + 0.5;
  // At rest this is exactly the source photograph with ordinary object-fit cover.
  // There is no permanent overscan, camera drift, colour grade or layer cutout.
  if (uZoom <= 1.0 && uParallax <= 0.0) {
    gl_FragColor = vec4(texture2D(uColor, original).rgb, 1.0);
    return;
  }
  vec2 at = uPivot + (original - uPivot) / uZoom;
  float depth = softDepth(at) - 0.5;
  vec2 edge = smoothstep(vec2(0.0), vec2(0.035), min(at, vec2(1.0) - at));
  vec2 warp = (uPivot - at) * depth * uParallax * edge.x * edge.y;
  warp = clamp(warp, vec2(-0.002), vec2(0.002));
  // The zoom is a convex UV interpolation. Limit even the tiny residual warp
  // to existing source pixels, so no screen corner can expose a hole.
  vec2 available = max(vec2(0.0), min(at, vec2(1.0) - at));
  at += clamp(warp, -available, available);
  gl_FragColor = vec4(texture2D(uColor, clamp(at, vec2(0.0), vec2(1.0))).rgb, 1.0);
}
