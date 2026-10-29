import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { RustHomeStage } from "../../../packages/runtime/src/index.ts";

test("base paths move the camera in metres while preserving its focal projection and release resources", async (t) => {
  const old = Object.getOwnPropertyDescriptor(globalThis, "document");
  Object.defineProperty(globalThis,"document",{configurable:true,value:{baseURI:"http://base.test/"}});
  t.after(()=>{if(old)Object.defineProperty(globalThis,"document",old);else Reflect.deleteProperty(globalThis,"document");});
  t.mock.method(globalThis,"fetch",async()=>new Response(JSON.stringify({
    version:1,calibration:{fov:55,aspect:1882/3344,camHeight:2,horizon:.3},
    textures:{clean:"clean.png",equipment:"original.webp"},
    meshes:[{name:"crown",material:"equipment",positions:[-1,0,-4,1,0,-4,0,2,-4],indices:[0,1,2]}],
  })));
  let texturesDisposed=0,rendererDisposed=0;
  t.mock.method(THREE.TextureLoader.prototype,"loadAsync",async()=>{
    const tex=new THREE.Texture();tex.addEventListener("dispose",()=>texturesDisposed++);return tex;
  });
  const shots:Array<{position:number[];projection:number[];meshes:string[]}> = [];
  class Renderer {
    capabilities={getMaxAnisotropy:()=>4};
    target:unknown=null;
    setClearColor(){} setPixelRatio(){} setSize(){}
    setRenderTarget(target:unknown){this.target=target;}
    getRenderTarget(){return this.target;}
    render(scene:THREE.Scene,camera:THREE.PerspectiveCamera){
      const meshes:string[]=[];scene.traverse(o=>{if(o instanceof THREE.Mesh)meshes.push(o.name);});
      shots.push({position:camera.position.toArray(),projection:camera.projectionMatrix.toArray(),meshes});
    }
    dispose(){rendererDisposed++;}
  }
  const attrs=new Map<string,string>();
  const canvas={clientWidth:390,clientHeight:844,setAttribute:(k:string,v:string)=>attrs.set(k,v)};
  const stage=new RustHomeStage({THREE:{...THREE,WebGLRenderer:Renderer}},canvas,"manifest.json");
  await stage.ready;
  stage.draw("base",0);const first=shots.at(-1)!;
  assert.deepEqual(first.position,[0,2,0]);assert.deepEqual(first.meshes,["crown"]);
  stage.draw("battle",.5);assert.ok(shots.at(-1)!.position[2]!<-1);
  stage.draw("battle",1);assert.deepEqual(shots.at(-1)!.position,[-.4,1.4,-2.55]);
  stage.draw("fitting",1);assert.ok(Math.abs(shots.at(-1)!.position[2]!+6.8)<1e-8);
  for(const shot of shots) assert.deepEqual(shot.projection,first.projection,"dolly never zooms the lens");
  assert.equal(attrs.get("data-home-target"),"fitting");
  const count=shots.length;stage.draw("fitting",1);assert.equal(shots.length,count,"idle frame is cached");
  stage.dispose();stage.dispose();stage.free();
  assert.equal(rendererDisposed,1);assert.equal(texturesDisposed,2);
});
