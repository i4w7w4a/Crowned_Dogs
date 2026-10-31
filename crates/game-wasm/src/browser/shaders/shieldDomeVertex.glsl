// The «Зонт» dome: a unit sphere cut at the floor, scaled to the fighter.
uniform float wideK;
uniform float wideY;
varying vec3 vLocal;
varying vec3 vNormal;
varying vec3 vView;

void main() {
  vLocal = position;
  vec4 mv = modelViewMatrix * vec4(position, 1.0);
  vView = mv.xyz;
  vNormal = normalize(normalMatrix * normal);
  gl_Position = projectionMatrix * mv;
  gl_Position.xy = mix(vec2(0.0, wideY) * gl_Position.w, gl_Position.xy, wideK);
}
