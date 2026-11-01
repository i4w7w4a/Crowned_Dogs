// White-phosphor image intensifier: lift the available dark detail, compress
// highlights and discard color. It does not reveal figures behind occluders.
float luminance = max(dot(c, vec3(0.2126, 0.7152, 0.0722)), 0.0);
float signal = pow(1.0 - exp(-luminance * 4.5), 0.52);
vec3 phosphor = vec3(0.79, 0.91, 0.87) * signal;
gl_FragColor = vec4(phosphor, max(a, n.a));
