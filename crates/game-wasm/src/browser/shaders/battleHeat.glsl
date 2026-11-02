// Heat acts only on distant, depth-tested architecture. Characters, HUD and
// near machinery stay stable; the sky (zero depth) is not warped.
vec2 backgroundUV = vUv;
float inverseDepth = hasDepth > 0.5 ? softInv(vUv, 0.03) : 0.0;
float distant = (1.0 - smoothstep(cdHeatDepth.y, cdHeatDepth.x, inverseDepth))
  * step(0.00001, inverseDepth) * hasDepth;
// strongest just over the horizon, where the light runs longest over hot ground;
// the ripples rise, the way hot air does
float band = exp(-pow((vUv.y - cdHeatDepth.w) / 0.12, 2.0));
float heat = cdHeatDepth.z * distant * (0.45 + 0.9 * band);
float rise = cdBattleTime * 1.6;
float drift = sin(vUv.y * 160.0 - rise + sin(vUv.x * 23.0 + cdBattleTime * 0.3) * 2.0)
  + 0.5 * sin(vUv.y * 340.0 - rise * 1.7 + vUv.x * 57.0);
float lift = sin(vUv.x * 120.0 + vUv.y * 90.0 - rise * 1.3);
backgroundUV += vec2(drift * 0.0011, lift * 0.0005) * heat;
vec3 bg = hasBg > 0.5 ? focused(clamp(backgroundUV, vec2(0.001), vec2(0.999))) : vec3(0.0);
