#ifdef USE_MAP
{
  // The scan's baked buzz cut under a pack hairstyle: painted in the hair's colour, its stubble kept as detail,
  // so no second, grey or blond hairline shows under the strips (mask: tools/hair-hd/scalp_mask.py).
  float cdHairMask = dot( texture2D( cdScalp, vMapUv ), cdScalpPick );
  float cdTexL = dot( sampledDiffuseColor.rgb, vec3( 0.2126, 0.7152, 0.0722 ) );
  diffuseColor.rgb = mix( diffuseColor.rgb, cdScalpTone * clamp( cdTexL / cdScalpLuma, 0.35, 1.8 ), cdHairMask );
}
#endif
