#ifdef USE_MAP
{
  // The slick's and the ponytail's own hairline (mask: tools/hair-hd/scalp_mask.py), 0.5 where it has nothing to say:
  // above, the hair's colour, solid under their strips and thinning out past the last roots in fine streaks along
  // the comb, so the strips' ends do not stand on bare skin; below, the scan's buzz on the forehead in front of
  // the hairline, taken back to the face's own skin, so no patch of stubble shows ahead of it.
  float cdLineS = ( dot( texture2D( cdHairline, vMapUv ), cdScalpPick ) * 2.0 - 1.0 ) * cdHairlineOn;
  float cdLine = max( cdLineS, 0.0 );
  float cdBare = max( -cdLineS, 0.0 );
  vec3 cdBuzzD = cdScalpBuzz - cdScalpSkin;
  float cdBuzzS = dot( sampledDiffuseColor.rgb - cdScalpSkin, cdBuzzD ) / max( dot( cdBuzzD, cdBuzzD ), 1e-6 );
  vec3 cdSkinCol = diffuse * mix( sampledDiffuseColor.rgb, cdScalpSkin, smoothstep( 0.05, 0.6, cdBuzzS ) );
  diffuseColor.rgb = mix( diffuseColor.rgb, cdSkinCol, cdBare );
  // The scan's baked buzz cut under a pack hairstyle: painted in the hair's colour, its stubble kept as detail,
  // so no second, grey or blond hairline shows under the strips (mask: the same script); under the slick's and
  // the ponytail's own hairline its flecks are evened out, or they showed as a grey or speckled patch ahead of the
  // strips and in the ponytail's parting.
  float cdHairMask = max( dot( texture2D( cdScalp, vMapUv ), cdScalpPick ) - cdBare, 0.0 );
  float cdTexL = dot( sampledDiffuseColor.rgb, vec3( 0.2126, 0.7152, 0.0722 ) );
  float cdRatio = clamp( cdTexL / cdScalpLuma, mix( 0.35, 0.75, cdLine ), mix( 1.8, 1.05, cdLine ) );
  diffuseColor.rgb = mix( diffuseColor.rgb, cdScalpTone * cdRatio, cdHairMask );
  diffuseColor.rgb = mix( diffuseColor.rgb, cdScalpTone, cdLine * ( 1.0 - cdHairMask ) );
  cdHairPaint = max( cdHairMask, cdLine );
}
#endif
