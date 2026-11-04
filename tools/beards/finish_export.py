"""Set stable alpha modes on Blender GLB exports; writes assets, never image pixels."""
import struct,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
for file in ['beards-male.glb','beards-authored.glb']:
 p=ROOT/'apps/web/packs'/file
 if not p.exists():continue
 b=p.read_bytes();size,typ=struct.unpack_from('<II',b,12);j=json.loads(b[20:20+size]);tail=b[20+size:]
 for m in j['materials']:
  m['doubleSided']=True
  if m['name'].startswith('M_Beard_Cards'):m['alphaMode']='MASK';m['alphaCutoff']=.2
  elif m['name'].startswith('M_Beard_Root'):m['alphaMode']='BLEND';m.pop('alphaCutoff',None)
 data=json.dumps(j,separators=(',',':')).encode();data+=b' '*((-len(data))%4)
 p.write_bytes(struct.pack('<III',0x46546c67,2,20+len(data)+len(tail))+struct.pack('<II',len(data),typ)+data+tail)
 print('PATCHED',file,p.stat().st_size)
