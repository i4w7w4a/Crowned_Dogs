
uniform sampler2D tComp;
uniform sampler2D tFig;
uniform float lod;
uniform float horizon;
uniform float threshold;
in vec2 vUv;
void main() {
  vec3 c = textureLod(tComp, vUv, lod).rgb;
  float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
  float sky = smoothstep(horizon - 0.1, horizon + 0.14, vUv.y); // vUv.y runs up from the bottom
  float m = smoothstep(threshold, 1.0, l) * sky * (1.0 - texture(tFig, vUv).a);
  gl_FragColor = vec4(c * m, 1.0);
}