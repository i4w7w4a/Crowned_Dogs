vec3 cdPomRay = cdPomOrthographic ? cdPomRayLocal : normalize(cdPomLocal - cdPomCameraLocal);
if (!cdPomIntersect(cdPomLocal, cdPomRay, cdPomHit)) discard;
cdPomUv = cdPomUV(cdPomHit);
vec4 cdPomViewHit = cdPomModelView * vec4(cdPomHit, 1.0);
cdPomViewPosition = -cdPomViewHit.xyz;
cdPomWorldHit = (cdPomModelWorld * vec4(cdPomHit, 1.0)).xyz;
cdPomWorldDelta = (cdPomModelWorld * vec4(cdPomHit - cdPomLocal, 0.0)).xyz;
vec4 cdPomClip = cdPomProjection * cdPomViewHit;
#ifdef USE_REVERSED_DEPTH_BUFFER
    cdPomDepth = cdPomClip.z / cdPomClip.w;
#else
    cdPomDepth = cdPomClip.z / cdPomClip.w * 0.5 + 0.5;
#endif
gl_FragDepth = cdPomDepth;
