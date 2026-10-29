
uniform sampler2D tSrc;
uniform vec2 light;
uniform float reach;
uniform float decay;
in vec2 vUv;
void main() {
  vec2 delta = (vUv - light) * reach / float(STEPS);
  vec2 p = vUv;
  vec3 sum = vec3(0.0);
  float w = 1.0, wsum = 0.0;
  for (int i = 0; i < STEPS; i++) {
    sum += texture(tSrc, p).rgb * w;
    wsum += w;
    w *= decay;
    p -= delta;
  }
  gl_FragColor = vec4(sum / wsum, 1.0);
}