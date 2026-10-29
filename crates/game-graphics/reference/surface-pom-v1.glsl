// Surface relief only: retain the authored mesh silhouette and geometric shadows.
// R = centered height, G = UV-island confidence/edge mask. No displacement is
// inferred from albedo and the existing tangent-space normal remains authoritative.
uniform sampler2D cdSurfaceHeight;
uniform float cdSurfaceDepth;
uniform float cdSurfaceMidpoint;
uniform int cdSurfaceSteps;
uniform int cdSurfaceRefine;
varying float cdSurfaceScale;

vec2 cdSurfaceReliefUv(vec2 uv, vec2 uvDx, vec2 uvDy, vec3 position, vec3 eye) {
    vec3 px = dFdx(position), py = dFdy(position);
    float uvDeterminant = uvDx.x * uvDy.y - uvDx.y * uvDy.x;
    // Degenerate UVs are not a meaningful height-field parameterization.
    if (abs(uvDeterminant) < 1e-12) return uv;
    vec3 pu = (px * uvDy.y - py * uvDx.y) / uvDeterminant;
    vec3 pv = (py * uvDx.x - px * uvDy.x) / uvDeterminant;
    vec3 n = normalize(cross(pu, pv));
    vec3 v = normalize(eye);
    if (dot(n, v) < 0.0) n = -n;
    float facing = abs(dot(n, v));
    // Curved silhouettes and almost edge-on rays keep the unmodified source UV.
    float grazing = smoothstep(0.16, 0.34, facing);
    vec2 baseSample = textureGrad(cdSurfaceHeight, uv, uvDx, uvDy).rg;
    float confidence = smoothstep(0.04, 0.85, baseSample.g);
    float depth = cdSurfaceDepth * cdSurfaceScale;
    float pixelWorld = max(length(px), length(py));
    float resolved = smoothstep(0.08, 0.45, depth / max(pixelWorld, 1e-8));
    depth *= grazing * confidence * resolved;
    if (depth < 1e-8 || cdSurfaceSteps == 0) return uv;

    vec3 dualU = cross(pv, n), dualV = cross(n, pu);
    float metric = dot(pu, dualU);
    if (abs(metric) < 1e-12) return uv;
    vec2 slope = vec2(dot(v, dualU), dot(v, dualV)) / metric / max(facing, 0.16);
    // Guard against an invalid island metric even when source UV derivatives exist.
    float totalOffset = length(slope * depth);
    if (totalOffset > 0.004) depth *= 0.004 / totalOffset;

    float top = (1.0 - cdSurfaceMidpoint) * depth;
    float bottom = -cdSurfaceMidpoint * depth;
    float previous = top, current = top;
    bool hit = false;
    for (int i = 1; i <= 40; ++i) {
        if (i > cdSurfaceSteps) break;
        current = mix(top, bottom, float(i) / float(cdSurfaceSteps));
        vec2 candidate = uv + slope * current;
        vec2 sampleValue = textureGrad(cdSurfaceHeight, candidate, uvDx, uvDy).rg;
        // Never let a ray traverse an uncovered UV island boundary.
        if (sampleValue.g < 0.025) return uv;
        float height = (sampleValue.r - cdSurfaceMidpoint) * depth;
        if (current <= height) { hit = true; break; }
        previous = current;
    }
    if (!hit) return uv;
    for (int i = 0; i < 6; ++i) {
        if (i >= cdSurfaceRefine) break;
        float middle = (previous + current) * 0.5;
        vec2 sampleValue = textureGrad(cdSurfaceHeight, uv + slope * middle, uvDx, uvDy).rg;
        if (sampleValue.g < 0.025) return uv;
        if (middle > (sampleValue.r - cdSurfaceMidpoint) * depth) previous = middle;
        else current = middle;
    }
    vec2 shifted = uv + slope * ((current + previous) * 0.5);
    float edge = smoothstep(0.04, 0.85, textureGrad(cdSurfaceHeight, shifted, uvDx, uvDy).g);
    return mix(uv, shifted, edge);
}
