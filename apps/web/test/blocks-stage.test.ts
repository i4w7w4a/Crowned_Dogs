import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const source = fileURLToPath(new URL("../public/img/blocks-stage/manifest.json", import.meta.url));
type Part = { name: string; material: string; positions: number[]; indices: number[]; occludesFigures: boolean };
const manifest = JSON.parse(readFileSync(source, "utf8")) as {
  version: number; calibration: { fov: number; aspect: number; camHeight: number; horizon: number };
  textures: Record<string,string>; textures2x: Record<string,string>; meshes: Part[];
  repairPolygons: number[][][]; reconstruction: { id: string; sourceSize: number[]; factoryMeshes: boolean };
};

test("the courtyard ships its own complete stage and resolves every material", () => {
  assert.equal(manifest.version, 1); assert.equal(manifest.reconstruction.id, "blocks-day");
  assert.equal(manifest.reconstruction.factoryMeshes, false);
  assert.deepEqual(manifest.reconstruction.sourceSize, [941,1672]);
  assert.equal(manifest.calibration.fov,55); assert.equal(manifest.calibration.camHeight,2);
  assert.ok(Math.abs(manifest.calibration.aspect-941/1672)<1e-10);
  assert.ok(Math.abs(manifest.calibration.horizon-(.608-2*.212/1.8))<1e-10);
  for (const path of [...Object.values(manifest.textures),...Object.values(manifest.textures2x)]) {
    assert.ok(existsSync(resolve(dirname(source),path)), `missing courtyard material ${path}`);
    assert.ok(!path.includes("battle-stage"),"the courtyard must not import factory materials");
  }
  for (const part of manifest.meshes) {
    assert.ok(part.positions.length>=9 && part.positions.length%3===0,part.name);
    assert.ok(part.positions.every(Number.isFinite),part.name);
    assert.ok(part.indices.length>=3 && part.indices.length%3===0,part.name);
    assert.ok(part.indices.every(index=>Number.isInteger(index)&&index>=0&&index<part.positions.length/3),part.name);
  }
  assert.ok(manifest.repairPolygons.length>0 && manifest.repairPolygons.length<=128);
  assert.ok(manifest.repairPolygons.every(polygon=>polygon.length>=3&&polygon.every(point=>point.length===2&&point.every(value=>value>=0&&value<=1))));
});

test("the bus has a closed three-dimensional hull rather than a cutout plane", () => {
  const parts=manifest.meshes.filter(part=>part.name.startsWith("Bus · closed damaged body"));
  assert.ok(parts.length>0);
  const edges=new Map<string,number>();
  let volume=0;
  for(const part of parts) {
    const at=(index:number)=>part.positions.slice(index*3,index*3+3) as [number,number,number];
    const key=(p:number[])=>p.join(",");
    for(let i=0;i<part.indices.length;i+=3) {
      const a=at(part.indices[i]!),b=at(part.indices[i+1]!),c=at(part.indices[i+2]!);
      volume+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6;
      for(const [p,q] of [[a,b],[b,c],[c,a]]) { const edge=[key(p!),key(q!)].sort().join("|");edges.set(edge,(edges.get(edge)??0)+1); }
    }
  }
  assert.ok([...edges.values()].every(count=>count===2),"every bus hull edge must meet exactly two triangles");
  assert.ok(Math.abs(volume)>4,`bus volume must be substantial, got ${volume}`);
  assert.ok(manifest.meshes.some(part=>part.name.startsWith("Bus wheel")));
});

test("courtyard ground and curved playground geometry preserve combat occlusion", () => {
  const floor=manifest.meshes.find(part=>part.name.startsWith("Courtyard ground"))!;
  assert.ok(floor); assert.equal(floor.occludesFigures,false);
  assert.ok(floor.positions.filter((_,i)=>i%3===1).every(y=>y===0));
  const hull=manifest.meshes.find(part=>part.name.startsWith("Playground rocket · cylindrical hull"))!;
  assert.ok(hull); assert.equal(hull.occludesFigures,true);
  assert.ok(new Set(hull.positions.filter((_,i)=>i%3===0)).size>=8);
  assert.ok(new Set(hull.positions.filter((_,i)=>i%3===2)).size>=8);
  const beams=manifest.meshes.filter(part=>/Swing|Rocket ladder/.test(part.name));
  assert.ok(beams.length>=8);
  assert.ok(beams.every(part=>part.occludesFigures&&part.indices.length>=12));
});
