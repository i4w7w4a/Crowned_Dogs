c = tone(n) + c * (1.0 - n.a);
// What sits on the lens itself lies over the gun and the hands as well.
{
  float wt = cdWeatherTime;
  float aspect = nearSize.x / max(nearSize.y, 1.0);
  vec2 q = vec2(vUv.x * aspect, vUv.y);
  // dust: a gust thickens until it swallows even the gun, and grains catch on the glass
  if (cdWeather2.y > 0.0) {
    float surge = cdWeather2.x;
#ifdef CD_WX_MINIMAL
    float roll = 0.5;
#else
    float roll = wxFbm(vec2(q.x * 1.6 - wt * 0.9, q.y * 2.4 + wt * 0.12));
#endif
    c = mix(c, cdDustColor * (0.88 + 0.12 * roll), clamp(surge * (0.3 + 0.25 * roll) * n.a, 0.0, 0.6));
    // out-of-focus specks: a few stuck on the glass, more blown past in a gust
    float specks = 0.0;
    for (int i = 0; i < CD_WX_LAYERS - 1; i++) {
      float fi = float(i);
      vec2 g = q * (7.0 + fi * 6.0);
      g.x -= wt * fi * 2.6;
      g.y += sin(g.x * 0.7 + fi) * 0.15 * fi;
      vec2 id = floor(g);
      float many = fi < 0.5 ? 0.12 * cdWeather2.y + 0.25 * surge : 0.05 * cdWeather2.y + 0.5 * surge;
      float show = clamp((wxHash(id + fi * 31.0) - (1.0 - many)) / 0.06, 0.0, 1.0);
      vec2 o = vec2(wxHash(id + 3.1), wxHash(id + 7.7)) * 0.5 + 0.25;
      float r = 0.07 + 0.15 * wxHash(id + 11.3);
      float d = length(fract(g) - o);
      // a lit disc with a slightly brighter rim, the way out-of-focus grit looks
      specks += smoothstep(r, r * 0.6, d) * (0.7 + 0.3 * smoothstep(r * 0.3, r * 0.85, d)) * show * (0.75 - fi * 0.25);
    }
    c = mix(c, cdDustColor * 1.18, clamp(specks * 0.32, 0.0, 0.55));
#ifndef CD_WX_MINIMAL
    // grains streaking past right in front of the lens
    vec2 sg = vec2(q.x * 38.0 - wt * 60.0, q.y * 210.0);
    vec2 sd = (fract(sg) - 0.5) * vec2(0.28, 2.0);
    float streak = step(0.994 - 0.012 * surge, wxHash(floor(sg))) * smoothstep(0.35, 0.0, length(sd));
    c = mix(c, cdDustColor * 1.1, streak * (0.15 + 0.45 * surge) * cdWeather2.y);
#endif
  }
  // rain: drops on the glass bend the view behind them; the gun and the hands are wet
  if (cdWeather2.z > 0.0) {
    float wet = cdWeather2.z;
    vec3 drop = wxDrops(q, 4.5, wt, 1.0, 0.4 * wet);
#ifndef CD_WX_MINIMAL // minimal quality: only the big drops, no running ones or beads
    vec3 d2 = wxDrops(q, 10.0, wt, 7.0, 0.5 * wet);
    vec3 sl = wxSlide(q, 7.0, wt, 21.0);
    if (d2.z > drop.z) drop = d2;
#ifndef CD_WX_LITE
    vec3 d3 = wxDrops(q, 23.0, wt, 13.0, 0.55 * wet);
    if (d3.z > drop.z) drop = d3;
#endif
    if (sl.z > drop.z) drop = sl;
#endif
    // wet metal and gloves: a little darker, with beads that catch the light
    float lit = dot(c, vec3(0.3, 0.5, 0.2));
    c *= 1.0 - 0.14 * wet * n.a;
#ifndef CD_WX_MINIMAL
    vec2 bq = q * 70.0;
    // beads sit only on the gun and the hands, and in one cell of eight
    if (n.a > 0.0 && wxHash(floor(bq) + 5.0) >= 0.88) {
      vec2 bd = (fract(bq) - 0.25 - 0.5 * vec2(wxHash(floor(bq) + 2.0), wxHash(floor(bq) + 3.0))) / (0.12 + 0.16 * wxHash(floor(bq) + 4.0));
      float bead = smoothstep(1.0, 0.3, length(bd));
      c += vec3(0.9, 0.94, 1.0) * bead * n.a * wet * (0.5 * lit + 0.05 + 0.8 * cdWeather.w);
    }
#endif
    if (drop.z > 0.001) {
      vec2 s = drop.xy / drop.z;
      float l2 = min(dot(s, s), 1.0);
      // a drop out of focus on the glass: a soft smudge of the view round it, a little bent
      vec2 ruv = clamp(vUv - s * vec2(0.03 / aspect, 0.03), vec2(0.001), vec2(0.999));
      vec3 seen = hasBg > 0.5 ? textureLod(tBg, ruv, 3.2).rgb * cdWeatherDim : c;
      seen = mix(seen, c * 0.95, n.a);
      vec3 nrm = vec3(s, sqrt(1.0 - l2));
      float glint = pow(max(dot(nrm, normalize(vec3(-0.45, 0.6, 0.66))), 0.0), 28.0);
      vec3 inside = seen * (1.0 - 0.3 * smoothstep(0.4, 1.0, l2)) // the edge bends some light away
        + vec3(0.9, 0.94, 1.0) * glint * (0.1 + 0.6 * dot(seen, vec3(0.33)) + 1.0 * cdWeather.w);
      c = mix(c, inside, drop.z * (0.6 + 0.3 * wet));
    }
  }
}
