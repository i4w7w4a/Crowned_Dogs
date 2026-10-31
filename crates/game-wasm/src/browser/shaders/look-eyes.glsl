#ifdef USE_MAP
{
  // Distance to the nearer iris, in iris radii.
  float cdD = min( distance( cdLocal, cdEyeA.xyz ) / cdEyeA.w, distance( cdLocal, cdEyeB.xyz ) / cdEyeB.w );
  vec3 cdS = pow( max( sampledDiffuseColor.rgb, vec3( 0.0 ) ), vec3( 1.0 / 2.2 ) );
  float cdSat = max( cdS.r, max( cdS.g, cdS.b ) ) - min( cdS.r, min( cdS.g, cdS.b ) );
  // Grey texels are the eye; the warmer skin of the lids keeps the skin tint.
  float cdGrey = 1.0 - smoothstep( 0.1, 0.18, cdSat );
  // The eyeball keeps the scan's own colour under any skin tint.
  diffuseColor.rgb = mix( diffuseColor.rgb, sampledDiffuseColor.rgb, ( 1.0 - smoothstep( 1.6, 2.2, cdD ) ) * cdGrey );
  // The iris glows in the picked colour, brightest at its middle.
  float cdIrisW = ( 1.0 - smoothstep( 0.7, 1.05, cdD ) ) * cdGrey;
  diffuseColor.rgb = mix( diffuseColor.rgb, cdIris * 0.25, cdIrisW );
  cdGlow = cdIris * cdIrisW * ( 1.3 + 1.7 * max( 1.0 - cdD, 0.0 ) );
}
#endif
