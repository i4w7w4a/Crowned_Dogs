uniform sampler2D photo;
uniform sampler2D surfaceTile;
uniform sampler2D originalPhoto;
uniform sampler2D repairMask;
uniform float useRepair;
uniform vec3 sideColor;
uniform vec3 sideTint;
uniform float sideExposure;
uniform float isSide;
uniform float surfaceKind;
uniform vec3 extensionTint;
uniform float extensionExposure;
uniform sampler2D bakedTexture;
uniform float useBakedUv;
varying vec4 sourceClip;
varying vec3 sourcePosition;
varying vec2 stageUv;
void main() {
  // Blender has already resolved projection, hidden surfaces and material seams.
  // The atlas uses the same display-referred bytes as the legacy photo path.
  if (useBakedUv > .5) {
    gl_FragColor = vec4(texture2D(bakedTexture, stageUv).rgb, 1.0);
    return;
  }
  float sourceW = sourceClip.w < 0.0 ? min(sourceClip.w, -.00001) : max(sourceClip.w, .00001);
  vec2 uv = sourceClip.xy / sourceW * .5 + .5;
  // Clamp to the clean plate at overscan edges, never reveal a transparent tear.
  vec3 color = isSide > .5 ? sideColor * sideTint * sideExposure : texture2D(photo, clamp(uv, vec2(.001), vec2(.999))).rgb;
  if (useRepair > .5) {
    vec2 at = clamp(uv, vec2(.001), vec2(.999));
    color = mix(texture2D(originalPhoto, at).rgb, color, texture2D(repairMask, at).r);
  }
  if (surfaceKind > .5) {
    // Independent 4-metre tiles extend the actual surfaces outside the source photograph.
    // They remain anchored to the original world, with no mirrored image or screen-space stretching.
    vec2 tileUv = surfaceKind > 1.5 && surfaceKind < 2.5 ? sourcePosition.zy * .25 : sourcePosition.xz * .25;
    vec3 extension = texture2D(surfaceTile, tileUv).rgb * extensionTint * extensionExposure;
    float photograph = smoothstep(0.0, .08, uv.y) * smoothstep(0.0, .08, 1.0 - uv.y)
      * smoothstep(0.0, .08, uv.x) * smoothstep(0.0, .08, 1.0 - uv.x) * step(0.0, sourceClip.w);
    color = mix(extension, color, photograph);
  }
  gl_FragColor = vec4(color, 1.0);
}
