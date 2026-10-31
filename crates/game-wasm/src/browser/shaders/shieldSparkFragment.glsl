// Field sparks are cold cyan, the bullet that goes off is hot amber; both add light.
uniform vec3 uCool;
uniform vec3 uWarm;
varying vec2 vCorner;
varying float vHeat;
varying float vWarm;

void main() {
  float across = 1.0 - abs(vCorner.y);
  float body = across * across * mix(0.1, 1.0, vCorner.x * vCorner.x) * vHeat;
  vec3 tint = mix(uCool, uWarm, vWarm);
  vec3 color = mix(tint, vec3(1.0), 0.35 * vHeat * vCorner.x) * body * (2.4 + 2.0 * vWarm);
  gl_FragColor = vec4(color, 0.0);
}
