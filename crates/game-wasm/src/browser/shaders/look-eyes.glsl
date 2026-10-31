#ifdef USE_MAP
{
  // The nearer iris, and where this texel sits on it: distance in iris radii, angle around the pupil.
  float cdA = distance( cdLocal, cdEyeA.xyz ) / cdEyeA.w;
  float cdB = distance( cdLocal, cdEyeB.xyz ) / cdEyeB.w;
  vec4 cdEye = cdA < cdB ? cdEyeA : cdEyeB;
  float cdD = min( cdA, cdB );
  vec2 cdQ = ( cdLocal.xy - cdEye.xy ) / cdEye.w;
  float cdAng = atan( cdQ.y, cdQ.x );
  vec3 cdS = pow( max( sampledDiffuseColor.rgb, vec3( 0.0 ) ), vec3( 1.0 / 2.2 ) );
  float cdSat = max( cdS.r, max( cdS.g, cdS.b ) ) - min( cdS.r, min( cdS.g, cdS.b ) );
  // Grey texels are the eye; the warmer skin of the lids keeps the skin tint.
  float cdGrey = 1.0 - smoothstep( 0.1, 0.18, cdSat );
  // The eyeball keeps the scan's own colour under any skin tint.
  diffuseColor.rgb = mix( diffuseColor.rgb, sampledDiffuseColor.rgb, ( 1.0 - smoothstep( 1.6, 2.2, cdD ) ) * cdGrey );
  // Slow breathing of the light; the pupil opens a little with it.
  float cdBreath = sin( cdTime * 1.6 );
  float cdPupil = 0.27 + 0.025 * cdBreath;
  float cdOpen = smoothstep( cdPupil - 0.05, cdPupil + 0.05, cdD );
  // Fine fibres that drift slowly around the pupil; the scan's own iris shading shows through.
  float cdFib = 0.5 + 0.28 * sin( cdAng * 29.0 + 1.4 * sin( cdAng * 5.0 + cdTime * 0.3 ) )
    + 0.22 * sin( cdAng * 47.0 - cdTime * 0.5 + cdD * 7.0 );
  float cdLum = dot( cdS, vec3( 0.299, 0.587, 0.114 ) );
  float cdIrisW = ( 1.0 - smoothstep( 0.84, 1.16, cdD ) ) * cdGrey;
  // Body of the iris, darker toward its rim like a real limbal ring.
  float cdBody = mix( 0.38, 0.88, cdFib ) * ( 1.0 - 0.6 * smoothstep( 0.6, 0.98, cdD ) ) * mix( 0.8, 1.2, cdLum );
  // A ring of light around the pupil, the hottest part, a little paler than the colour.
  float cdRing = exp( -pow( ( cdD - cdPupil - 0.11 ) / 0.085, 2.0 ) );
  vec3 cdHot = mix( cdIris, vec3( 1.0 ), 0.2 );
  vec3 cdLight = cdIris * cdBody * cdOpen + cdHot * cdRing * 0.55 + cdIris * ( 1.0 - cdOpen ) * 0.08;
  // A soft halo over the white of the eye and, much fainter, the lids.
  float cdHalo = exp( -max( cdD - 0.9, 0.0 ) * 2.1 ) * ( 1.0 - cdIrisW ) * mix( 0.05, 0.42, cdGrey );
  diffuseColor.rgb = mix( diffuseColor.rgb, cdIris * 0.06, cdIrisW );
  cdGlow = ( cdLight * cdIrisW + cdIris * cdHalo * step( cdD, 3.5 ) ) * ( 0.9 + 0.1 * cdBreath );
}
#endif
