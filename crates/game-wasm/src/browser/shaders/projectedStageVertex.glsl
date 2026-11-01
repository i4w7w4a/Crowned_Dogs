uniform mat4 sourceViewProjection;
varying vec4 sourceClip;
varying vec3 sourcePosition;
varying float viewDepth;
varying vec2 stageUv;
void main() {
  stageUv = uv;
  // Project in the original world, BEFORE the stage translates. Texture stays glued to geometry.
  sourceClip = sourceViewProjection * vec4(position, 1.0);
  sourcePosition = position;
  vec4 eye = modelViewMatrix * vec4(position, 1.0);
  viewDepth = -eye.z;
  gl_Position = projectionMatrix * eye;
}
