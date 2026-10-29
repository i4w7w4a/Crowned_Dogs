varying float viewDepth;
void main() { gl_FragColor = vec4(vec3(clamp(.8 / max(viewDepth, .8), 0.0, 1.0)), 1.0); }
