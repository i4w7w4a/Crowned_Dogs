#if defined(USE_SHADOWMAP) && NUM_DIR_LIGHT_SHADOWS > 0
uniform mat4 directionalShadowMatrix[NUM_DIR_LIGHT_SHADOWS];
#endif
#if defined(USE_SHADOWMAP) && NUM_POINT_LIGHT_SHADOWS > 0
uniform mat4 pointShadowMatrix[NUM_POINT_LIGHT_SHADOWS];
#endif
#if NUM_SPOT_LIGHT_COORDS > 0
uniform mat4 spotLightMatrix[NUM_SPOT_LIGHT_COORDS];
#endif
