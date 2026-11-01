uniform vec4 cdWeather;
uniform float cdWeatherTime;
uniform float cdWeatherDim;
uniform vec3 cdFogColor;
uniform vec3 cdDustColor;
uniform vec3 cdRainColor;
float wxHash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float wxNoise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(wxHash(i), wxHash(i + vec2(1.0, 0.0)), f.x), mix(wxHash(i + vec2(0.0, 1.0)), wxHash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float wxFbm(vec2 p) { return 0.55 * wxNoise(p) + 0.3 * wxNoise(p * 2.03 + 1.7) + 0.15 * wxNoise(p * 4.1 + 5.2); }
