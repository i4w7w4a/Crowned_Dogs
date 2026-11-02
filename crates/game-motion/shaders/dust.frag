uniform vec3 color;
varying float vAlpha;
varying float vLight;
void main() {
  float r = length(gl_PointCoord - 0.5) * 2.0;
  float a = vAlpha * smoothstep(1.0, 0.55, r);
  if (a < 0.002) discard;
  gl_FragColor = vec4(color * vLight * a, 0.0); // adds light, covers nothing
}
