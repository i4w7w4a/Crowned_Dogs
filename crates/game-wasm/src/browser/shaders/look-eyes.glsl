#ifdef USE_MAP
{
  vec2 cdUv = vMapUv;
  float cdInA = step( cdEyeA.x, cdUv.x ) * step( cdUv.x, cdEyeA.z ) * step( cdEyeA.y, cdUv.y ) * step( cdUv.y, cdEyeA.w );
  float cdInB = step( cdEyeB.x, cdUv.x ) * step( cdUv.x, cdEyeB.z ) * step( cdEyeB.y, cdUv.y ) * step( cdUv.y, cdEyeB.w );
  float cdEye = max( cdInA, cdInB );
  // Eyes keep the scan's own colour under any skin tint.
  diffuseColor.rgb = mix( diffuseColor.rgb, sampledDiffuseColor.rgb, cdEye );
  vec3 cdS = pow( max( sampledDiffuseColor.rgb, vec3( 0.0 ) ), vec3( 1.0 / 2.2 ) );
  float cdL = dot( cdS, vec3( 0.299, 0.587, 0.114 ) );
  float cdSat = max( cdS.r, max( cdS.g, cdS.b ) ) - min( cdS.r, min( cdS.g, cdS.b ) );
  // Grey, mid-dark texels inside an eye area are the iris.
  float cdIrisW = cdEye * cdIrisOn * ( 1.0 - smoothstep( 0.07, 0.14, cdSat ) ) * ( 1.0 - smoothstep( 0.5, 0.68, cdL ) ) * smoothstep( 0.06, 0.14, cdL );
  diffuseColor.rgb = mix( diffuseColor.rgb, cdIris * pow( cdL / 0.33, 2.2 ), cdIrisW );
}
#endif
