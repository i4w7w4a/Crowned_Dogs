// Finite-volume relief mapping: unlike UV-only POM, an empty ray discards the
// proxy fragment. Colour and shadow passes use the same intersection and depth.
varying vec3 cdPomLocal;
uniform sampler2D cdPomRelief;
uniform vec2 cdPomSize; // radius, half-height in metres
uniform vec3 cdPomCameraLocal;
uniform vec3 cdPomRayLocal;
uniform bool cdPomOrthographic;
uniform int cdPomSteps;
uniform int cdPomRefine;
uniform float cdPomTexel;
uniform mat4 cdPomModelView;
uniform mat4 cdPomModelWorld;
uniform mat4 cdPomProjection;
uniform mat3 cdPomNormalMatrix;
uniform mat3 cdPomViewToLocal;
vec3 cdPomHit;
vec2 cdPomUv;
vec3 cdPomViewPosition;
vec3 cdPomWorldDelta;
vec3 cdPomWorldHit;
float cdPomDepth;

vec2 cdPomUV(vec3 p) { return p.xz / (2.0 * cdPomSize.x) + 0.5; }

float cdPomField(vec3 p) {
    vec4 relief = textureLod(cdPomRelief, clamp(cdPomUV(p), vec2(0.0), vec2(1.0)), 0.0);
    float top = (relief.r * 2.0 - 1.0) * cdPomSize.y;
    float edge = length(p.xz) - (cdPomSize.x - relief.g * 0.0035);
    return max(max(p.y - top, edge), -p.y - cdPomSize.y);
}

vec3 cdPomNormal(vec3 p) {
    float e = max(0.00025, cdPomTexel * cdPomSize.x);
    return normalize(vec3(
        cdPomField(p + vec3(e,0.0,0.0)) - cdPomField(p - vec3(e,0.0,0.0)),
        cdPomField(p + vec3(0.0,e,0.0)) - cdPomField(p - vec3(0.0,e,0.0)),
        cdPomField(p + vec3(0.0,0.0,e)) - cdPomField(p - vec3(0.0,0.0,e))
    ));
}

bool cdPomIntersect(vec3 surface, vec3 ray, out vec3 hit) {
    vec3 origin = surface - ray * 0.0001;
    vec3 halfBox = vec3(cdPomSize.x, cdPomSize.y, cdPomSize.x);
    vec3 safeRay = vec3(abs(ray.x) < 1e-7 ? 1e-7 : ray.x, abs(ray.y) < 1e-7 ? 1e-7 : ray.y, abs(ray.z) < 1e-7 ? 1e-7 : ray.z);
    vec3 a = (-halfBox - origin) / safeRay;
    vec3 b = ( halfBox - origin) / safeRay;
    vec3 entry = min(a, b), exitPoint = max(a, b);
    float nearT = max(0.0, max(max(entry.x, entry.y), entry.z));
    float farT = min(min(exitPoint.x, exitPoint.y), exitPoint.z);
    if (nearT > farT) return false;
    // Bounding cylinder shortens grazing rays before sampling the finer chipped rim.
    float qa = dot(ray.xz, ray.xz);
    if (qa > 1e-8) {
        float qb = dot(origin.xz, ray.xz);
        float qc = dot(origin.xz, origin.xz) - cdPomSize.x * cdPomSize.x;
        float discriminant = qb * qb - qa * qc;
        if (discriminant < 0.0) return false;
        float root = sqrt(discriminant);
        nearT = max(nearT, (-qb - root) / qa);
        farT = min(farT, (-qb + root) / qa);
        if (nearT > farT) return false;
    }
    hit = origin + ray * nearT;
    if (cdPomField(hit) <= 0.000001) return true;
    float previous = nearT;
    for (int i = 1; i <= 72; ++i) {
        if (i > cdPomSteps) break;
        float t = mix(nearT, farT, float(i) / float(cdPomSteps));
        vec3 p = origin + ray * t;
        if (cdPomField(p) <= 0.0) {
            float low = previous, high = t;
            for (int j = 0; j < 7; ++j) {
                if (j >= cdPomRefine) break;
                float middle = (low + high) * 0.5;
                if (cdPomField(origin + ray * middle) > 0.0) low = middle;
                else high = middle;
            }
            hit = origin + ray * ((low + high) * 0.5);
            return true;
        }
        previous = t;
    }
    return false;
}

// Relief-scale occlusion supplements the ordinary shadow map; large shadows still
// come from Three's actual scene lights and are received at the corrected point.
float cdPomLightVisibility(vec3 viewDirection) {
    vec3 direction = normalize(cdPomViewToLocal * viewDirection);
    vec3 origin = cdPomHit + cdPomNormal(cdPomHit) * 0.00035;
    float distance = 0.00065;
    for (int i = 0; i < 10; ++i) {
        vec3 p = origin + direction * distance;
        if (p.y > cdPomSize.y || length(p.xz) > cdPomSize.x) break;
        if (cdPomField(p) < -0.00008) return 0.12;
        distance *= 1.65;
    }
    return 1.0;
}
