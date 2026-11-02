// Independent Three.js oracle for the Rust camera and spline migration.
import assert from "node:assert/strict";
import * as THREE from "three";
import { callRust } from "../../../packages/runtime/src/index.ts";

for (const [width, height] of [[320,480],[360,640],[390,844],[480,760],[1280,720],[1920,1080]]) {
  const camera = new THREE.PerspectiveCamera(33, width / height, .05, 40);
  const tan = Math.tan(THREE.MathUtils.degToRad(camera.fov) / 2);
  let distance = Math.max(1.12 / tan, .86 / (tan * camera.aspect));
  const outline = [new THREE.Vector3(-.2,1.95,.2),new THREE.Vector3(.2,1.95,.2)];
  for (let i=0;i<16;i++) {
    const a=i/16*Math.PI*2;
    outline.push(new THREE.Vector3(Math.cos(a)*.81,0,Math.sin(a)*.81),new THREE.Vector3(Math.cos(a)*.63,1.1,Math.sin(a)*.63));
  }
  for (let i=0;i<60;i++) {
    camera.position.set(0,1.36,distance); camera.lookAt(0,.88,0); camera.updateMatrixWorld();
    if(outline.every(point=>{const p=point.clone().project(camera);return Math.abs(p.x)<.94&&Math.abs(p.y)<1-24/height;})) break;
    distance*=1.015;
  }
  const fit=callRust("graphics.fittingCamera",{width,height,fov:33});
  assert.ok(Math.abs(distance-fit.distance)<1e-11,`camera fit ${width}x${height}`);
  for(const [x,y] of [[0,0],[13,-8],[-21,15]]) {
    camera.position.set(0,1.36,distance); camera.lookAt(0,.88,0);
    const k=height/2/tan/Math.hypot(distance,.48);
    camera.translateX(-x/k);camera.translateY(y/k);camera.updateMatrixWorld();
    const aim=callRust("graphics.fittingAim",{distance,height,fov:33,x,y});
    assert.ok(camera.position.distanceTo(new THREE.Vector3(...aim.position))<1e-11,`camera drift ${width}x${height}`);
  }
}

for(const pistol of [false,true]) for(const roll of [-.96,-Math.PI/2,.2]) {
  const mount=new THREE.Object3D(); mount.position.set(-.035,1.18,.215); mount.rotation.z=roll; mount.updateMatrix();
  const at=(p)=>p.applyMatrix4(mount.matrix);
  const points=pistol?[at(new THREE.Vector3(-.025,-.03,0)),new THREE.Vector3(.24,1.02,.18),new THREE.Vector3(.24,1.08,-.12)]:[
    at(new THREE.Vector3(-.24,.08,0)),new THREE.Vector3(-.21,1.50,.20),new THREE.Vector3(-.22,1.53,-.08),new THREE.Vector3(.13,1.35,-.205),new THREE.Vector3(.25,1.12,-.10),new THREE.Vector3(.25,1.06,.17),at(new THREE.Vector3(.34,.065,0))
  ];
  const curve=new THREE.CatmullRomCurve3(points,false,"centripetal"),expected=[];
  for(let i=0;i<=56;i++){
    const t=i/56,p=curve.getPoint(t),tangent=curve.getTangent(t);
    const outward=new THREE.Vector3(p.x,0,p.z).normalize();if(outward.lengthSq()<.1)outward.set(0,0,1);
    const across=new THREE.Vector3().crossVectors(tangent,outward).normalize().multiplyScalar((pistol?.018:.031)/2);
    expected.push(...p.clone().add(across).toArray(),...p.clone().sub(across).toArray());
  }
  const result=callRust("graphics.sling",{matrix:mount.matrix.elements,pistol});
  assert.equal(result.position.length,expected.length);
  assert.ok(Math.max(...expected.map((v,i)=>Math.abs(v-result.position[i])))<1e-10,`sling geometry ${pistol} ${roll}`);
}
console.log("Rust graphics parity: 6 camera viewports, 18 camera shifts, 6 complete sling meshes passed.");
