"""Blender 4.5 / bpy. Sky visibility in the ACTUAL exported architecture, used only for
indirect light. Re-bake after changing permanent walls, roofs or portals. RGBA stores
four heights rather than colour; the native shader interpolates the receiving point.
    blender-4.5 -b --python-exit-code 1 --python tools/bake_sky_shelter.py
"""
from pathlib import Path
import math,struct,zlib
import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

root=Path(__file__).resolve().parents[1]; n=128
heights=(.10,1.30,2.65,4.25);ray_count=24
rays=[]
for i in range(ray_count):
    r=math.sqrt((i+.5)/ray_count);a=i*2.3999632297
    rays.append(Vector((r*math.cos(a),r*math.sin(a),math.sqrt(1-r*r))))

def save(path,rows):
    def chunk(tag,data):return struct.pack('>I',len(data))+tag+data+struct.pack('>I',zlib.crc32(tag+data)&0xffffffff)
    hdr=struct.pack('>2I5B',n,n,8,6,0,0,0)
    path.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',hdr)+chunk(b'IDAT',zlib.compress(b''.join(b'\x00'+row for row in rows),9))+chunk(b'IEND',b''))

for src,out in [('lanes/orchard.glb','square'),('cistern/cistern.glb','cistern')]:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(root/'assets'/src));bpy.context.view_layer.update()
    verts=[];faces=[]
    for obj in bpy.context.scene.objects:
        if obj.type!='MESH':continue
        # Foliage has alpha holes, and the tool box and handwheel can move. Those
        # cast normal live shadows; they do not roof in the entire probe cell.
        lineage=[];p=obj
        while p is not None:lineage.append(p.name.lower());p=p.parent
        if any(any(tok in name for tok in ('canopy_','ripe_plum','wheel_pickup','tonic_lid','tool_lid','sprite_')) for name in lineage):continue
        mesh=obj.data;start=len(verts);verts.extend(obj.matrix_world@v.co for v in mesh.vertices)
        faces.extend(tuple(start+i for i in poly.vertices) for poly in mesh.polygons)
    tree=BVHTree.FromPolygons(verts,faces);rows=[]
    for iz in range(n):
        row=bytearray();z=(iz+.5)/4.-16.
        for ix in range(n):
            x=(ix+.5)/4.-16.
            for h in heights:
                at=Vector((x,-z,h));total=0.
                for ray in rays:
                    dist=tree.ray_cast(at,ray,8.)[3]
                    total+=1. if dist is None else .06+.48*(dist/8.)**2
                row.append(round(255*(.14+.86*total/ray_count)))
        rows.append(bytes(row))
    save(root/'assets/inks'/f'{out}-shelter.png',rows)
    print('Sky shelter:',src,len(verts),len(faces),n*n*len(heights)*ray_count,'rays')
