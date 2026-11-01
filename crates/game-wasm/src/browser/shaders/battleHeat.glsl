// Heat acts only on distant, depth-tested architecture. Characters, HUD and
// near machinery stay stable; the sky (zero depth) is not warped.
vec2 backgroundUV = vUv;
float inverseDepth = texture(tDepth, vUv).r * 1.25;
float distant = (1.0 - smoothstep(cdHeatDepth.y, cdHeatDepth.x, inverseDepth))
  * step(0.00001, inverseDepth) * hasDepth;
float drift = sin(vUv.y * 145.0 + cdBattleTime * 1.3)
  + 0.45 * sin(vUv.y * 311.0 - cdBattleTime * 0.9);
backgroundUV += vec2(drift * 0.00055, sin(vUv.x * 91.0 + cdBattleTime) * 0.00012) * distant;
vec3 bg = hasBg > 0.5 ? focused(clamp(backgroundUV, vec2(0.001), vec2(0.999))) : vec3(0.0);
