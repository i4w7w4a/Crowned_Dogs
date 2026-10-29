void main() {
vec2 cdSurfaceDx = dFdx(vNormalMapUv), cdSurfaceDy = dFdy(vNormalMapUv);
vec2 cdSurfaceUv = cdSurfaceReliefUv(vNormalMapUv, cdSurfaceDx, cdSurfaceDy, -vViewPosition, vViewPosition);
