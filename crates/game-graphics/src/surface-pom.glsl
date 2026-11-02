// Metric surface POM. The mesh owns the outline; every PBR map samples one hit.
// R: combined height, G: UV coverage, B: authored detail height, A: reserved.
uniform sampler2D cdSurfaceHeight;
uniform float cdSurfaceDepth;
uniform float cdSurfaceDetailDepth;
uniform float cdSurfaceFabricDepth;
uniform float cdSurfaceFabricPeriod;
uniform float cdSurfaceMidpoint;
uniform int cdSurfaceSteps;
uniform int cdSurfaceRefine;
uniform int cdSurfaceShadowSteps;
varying float cdSurfaceScale;
varying vec3 cdSurfaceLocal;
vec3 cdSurfaceDualU, cdSurfaceDualV, cdSurfaceN;
float cdSurfaceMetric, cdSurfaceActiveDepth, cdSurfaceDetailScale, cdSurfaceHitHeight;
vec2 cdSurfaceHitUv, cdSurfaceDx, cdSurfaceDy;
vec3 cdSurfaceLocalU, cdSurfaceLocalV, cdSurfaceFabricWeights;
vec2 cdSurfaceBaseUv;
float cdSurfaceFabricFilter;

float cdSurfaceWeave(vec2 p) {
    vec2 cell=p/max(cdSurfaceFabricPeriod,1e-6);
    cell+=0.025*vec2(sin(cell.y*0.71),sin(cell.x*0.83));
    vec2 across=(fract(cell)-0.5)/0.47;
    vec2 threads=sqrt(max(vec2(0.0),vec2(1.0)-across*across));
    float over=0.12*cos(cell.x*3.14159265)*cos(cell.y*3.14159265);
    float yarn=max(0.40*threads.x+over,0.40*threads.y-over)-0.34;
    float fibres=0.012*(sin(cell.x*31.4159265)*threads.x+sin(cell.y*31.4159265)*threads.y);
    return yarn+fibres;
}
float cdSurfaceFabric(vec2 uv) {
    if(cdSurfaceFabricDepth<=0.0) return 0.0;
    vec3 p=cdSurfaceLocal+cdSurfaceLocalU*(uv.x-cdSurfaceBaseUv.x)+cdSurfaceLocalV*(uv.y-cdSurfaceBaseUv.y);
    float eligible=textureGrad(cdSurfaceHeight,uv,cdSurfaceDx,cdSurfaceDy).a;
    return dot(cdSurfaceFabricWeights,vec3(cdSurfaceWeave(p.yz),cdSurfaceWeave(p.xz),cdSurfaceWeave(p.xy)))*cdSurfaceFabricFilter*eligible;
}

vec2 cdSurfaceSample(vec2 uv) {
    if(any(lessThan(uv,vec2(0.0))) || any(greaterThan(uv,vec2(1.0)))) return vec2(cdSurfaceMidpoint,0.0);
    vec2 field=textureGrad(cdSurfaceHeight, uv, cdSurfaceDx, cdSurfaceDy).rg;
    field.r+=cdSurfaceFabric(uv)*cdSurfaceFabricDepth/max(cdSurfaceDepth,1e-8);
    return field;
}
vec2 cdSurfaceSlope(vec3 direction) {
    return vec2(dot(direction,cdSurfaceDualU),dot(direction,cdSurfaceDualV)) / cdSurfaceMetric;
}
vec2 cdSurfaceReliefUv(vec2 uv, vec2 uvDx, vec2 uvDy, vec3 position, vec3 eye) {
    cdSurfaceDx=uvDx; cdSurfaceDy=uvDy; cdSurfaceHitUv=uv;
    cdSurfaceBaseUv=uv;
    cdSurfaceActiveDepth=0.0; cdSurfaceDetailScale=0.0; cdSurfaceHitHeight=0.0;
    vec3 px=dFdx(position), py=dFdy(position);
    float determinant=uvDx.x*uvDy.y-uvDx.y*uvDy.x;
    if (abs(determinant)<1e-12) return uv;
    vec3 pu=(px*uvDy.y-py*uvDx.y)/determinant;
    vec3 pv=(py*uvDx.x-px*uvDy.x)/determinant;
    vec3 localX=dFdx(cdSurfaceLocal),localY=dFdy(cdSurfaceLocal);
    cdSurfaceLocalU=(localX*uvDy.y-localY*uvDx.y)/determinant;
    cdSurfaceLocalV=(localY*uvDx.x-localX*uvDy.x)/determinant;
    cdSurfaceFabricWeights=pow(abs(normalize(cross(cdSurfaceLocalU,cdSurfaceLocalV))),vec3(6.0));
    cdSurfaceFabricWeights/=max(dot(cdSurfaceFabricWeights,vec3(1.0)),1e-8);
    float footprint=max(length(localX),length(localY))/max(cdSurfaceFabricPeriod,1e-6);
    cdSurfaceFabricFilter=1.0-smoothstep(0.25,0.65,footprint);
    vec3 v=normalize(eye);
    cdSurfaceN=normalize(cross(pu,pv));
    if (dot(cdSurfaceN,v)<0.0) cdSurfaceN=-cdSurfaceN;
    cdSurfaceDualU=cross(pv,cdSurfaceN); cdSurfaceDualV=cross(cdSurfaceN,pu);
    cdSurfaceMetric=dot(pu,cdSurfaceDualU);
    if (abs(cdSurfaceMetric)<1e-12) return uv;
    vec2 sampleValue=cdSurfaceSample(uv);
    float facing=abs(dot(cdSurfaceN,v));
    float fade=smoothstep(0.035,0.10,facing)*smoothstep(0.01,0.30,sampleValue.g);
    float depth=cdSurfaceDepth*cdSurfaceScale;
    fade*=smoothstep(0.015,0.08,depth/max(max(length(px),length(py)),1e-8));
    depth*=fade;
    if (depth<1e-8 || cdSurfaceSteps==0) return uv;
    vec2 slope=cdSurfaceSlope(v)/max(facing,0.035);
    // Coverage fades the relief once; search never borrows another UV island.
    cdSurfaceActiveDepth=depth;
    cdSurfaceDetailScale=cdSurfaceScale*fade;
    float top=(1.0-cdSurfaceMidpoint)*depth, bottom=-cdSurfaceMidpoint*depth;
    float previous=top, current=top;
    bool hit=false;
    for(int i=1;i<=64;++i) {
        if(i>cdSurfaceSteps) break;
        current=mix(top,bottom,float(i)/float(cdSurfaceSteps));
        sampleValue=cdSurfaceSample(uv+slope*current);
        if(sampleValue.g<0.012) {cdSurfaceActiveDepth=0.0;cdSurfaceDetailScale=0.0;return uv;}
        if(current<=(sampleValue.r-cdSurfaceMidpoint)*depth) {hit=true;break;}
        previous=current;
    }
    if(!hit) {cdSurfaceActiveDepth=0.0;cdSurfaceDetailScale=0.0;return uv;}
    for(int i=0;i<7;++i) {
        if(i>=cdSurfaceRefine) break;
        float middle=(previous+current)*0.5;
        sampleValue=cdSurfaceSample(uv+slope*middle);
        if(sampleValue.g<0.012) {cdSurfaceActiveDepth=0.0;cdSurfaceDetailScale=0.0;return uv;}
        if(middle>(sampleValue.r-cdSurfaceMidpoint)*depth) previous=middle;else current=middle;
    }
    cdSurfaceHitHeight=(current+previous)*0.5;
    cdSurfaceHitUv=uv+slope*cdSurfaceHitHeight;
    return cdSurfaceHitUv;
}

// Only newly authored relief adds a normal perturbation. The source normal map
// already contains the recovered component, so it must not be applied twice.
float cdSurfaceDetail(vec2 uv) {
    float detail=textureGrad(cdSurfaceHeight,uv,cdSurfaceDx,cdSurfaceDy).b-cdSurfaceMidpoint;
    return detail*cdSurfaceDetailDepth+cdSurfaceFabric(uv)*cdSurfaceFabricDepth;
}
vec3 cdSurfaceDetailNormal(vec3 normal) {
    if(cdSurfaceDetailScale<=0.0) return normal;
    vec2 texel=1.0/vec2(textureSize(cdSurfaceHeight,0));
    if(cdSurfaceFabricDepth>0.0) texel=min(texel,vec2(cdSurfaceFabricPeriod*0.12)/max(vec2(length(cdSurfaceLocalU),length(cdSurfaceLocalV)),vec2(1e-7)));
    float u0=cdSurfaceDetail(cdSurfaceHitUv-vec2(texel.x,0));
    float u1=cdSurfaceDetail(cdSurfaceHitUv+vec2(texel.x,0));
    float v0=cdSurfaceDetail(cdSurfaceHitUv-vec2(0,texel.y));
    float v1=cdSurfaceDetail(cdSurfaceHitUv+vec2(0,texel.y));
    vec2 gradient=vec2((u1-u0)/texel.x,(v1-v0)/texel.y)*0.5*cdSurfaceDetailScale;
    vec3 surfaceGradient=(cdSurfaceDualU*gradient.x+cdSurfaceDualV*gradient.y)/cdSurfaceMetric;
    return normalize(normal-surfaceGradient);
}

// Height-field visibility is evaluated independently for each real scene light.
// It modulates direct light only; ambient/environment lighting is never blackened.
float cdSurfaceLightVisibility(vec3 direction) {
    if(cdSurfaceActiveDepth<=0.0 || cdSurfaceShadowSteps==0) return 1.0;
    vec3 l=normalize(direction);
    float facing=dot(cdSurfaceN,l);
    if(facing<=0.025) return 1.0;
    float top=(1.0-cdSurfaceMidpoint)*cdSurfaceActiveDepth;
    float remaining=max(0.0,top-cdSurfaceHitHeight);
    vec2 slope=cdSurfaceSlope(l)/facing;
    float occlusion=0.0;
    for(int i=1;i<=16;++i) {
        if(i>cdSurfaceShadowSteps) break;
        float rise=remaining*float(i)/float(cdSurfaceShadowSteps);
        vec2 sampleValue=cdSurfaceSample(cdSurfaceHitUv+slope*rise);
        if(sampleValue.g<0.025) break;
        float obstacle=(sampleValue.r-cdSurfaceMidpoint)*cdSurfaceActiveDepth;
        float clearance=obstacle-(cdSurfaceHitHeight+rise);
        float bias=cdSurfaceActiveDepth*0.018;
        occlusion=max(occlusion,smoothstep(bias,bias+cdSurfaceActiveDepth*0.06,clearance));
    }
    return 1.0-0.88*occlusion;
}
