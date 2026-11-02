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
  if (uZoom <= 1.0 && uParallax <= 0.0 && uLive <= 0.0) {
    gl_FragColor = vec4(texture2D(uColor, original).rgb, 1.0);
    return;
  }
  // A live scene is held by hand: a 2.4% margin lets the camera breathe without showing an edge.
  float t = uTime;
  vec2 sway = vec2(sin(t * 0.31) + 0.45 * sin(t * 0.83 + 1.3),
                   0.7 * sin(t * 0.23 + 0.6) + 0.35 * sin(t * 1.4)) * uLive;
  vec2 at = uPivot + (original - uPivot) / (uZoom * (1.0 + 0.024 * uLive)) + sway * 0.0018;
  float depth = softDepth(at);
  vec2 edge = smoothstep(vec2(0.0), vec2(0.035), min(at, vec2(1.0) - at));
  vec2 warp = (uPivot - at) * (depth - 0.5) * uParallax * edge.x * edge.y;
  warp = clamp(warp, vec2(-0.002), vec2(0.002));
  // Near things swing more than far ones as the camera breathes.
  warp += sway * 0.006 * (depth - 0.5) * edge.x * edge.y;
  vec2 picture = vec2(at.x, 1.0 - at.y);
  vec2 life = cdLiveWarp(picture, t) * uLive;
  warp += vec2(life.x, -life.y);
  // The zoom is a convex UV interpolation. Limit even the tiny residual warp
  // to existing source pixels, so no screen corner can expose a hole.
  vec2 available = max(vec2(0.0), min(at, vec2(1.0) - at));
  at += clamp(warp, -available, available);
  at = clamp(at, vec2(0.0), vec2(1.0));
  vec3 col = texture2D(uColor, at).rgb;
  if (uLive > 0.0) {
    col = mix(col, cdLiveLight(col, vec2(at.x, 1.0 - at.y), depth, t), uLive);
  }
  gl_FragColor = vec4(col, 1.0);
}
