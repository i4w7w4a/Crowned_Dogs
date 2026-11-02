#include <lights_physical_fragment>
{
  vec3 nDx = dFdx( normal ), nDy = dFdy( normal );
  float kernel = min( 0.5 * ( dot( nDx, nDx ) + dot( nDy, nDy ) ), 0.18 );
  material.roughness = sqrt( clamp( material.roughness * material.roughness + kernel, 0.0, 1.0 ) );
}
