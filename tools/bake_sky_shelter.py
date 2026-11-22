"""Bake indirect light from actual GLB geometry (Blender 4.5). Interior RGBA stores
sky visibility at four heights. The sunny square uses one horizontal tile per
height: R = sky, GBA = one diffuse return from sunlit paving and masonry (linear).
    blender-4.5 -b --python-exit-code 1 --python tools/bake_sky_shelter.py
"""
from pathlib import Path
import math,struct,zlib,sys,json
import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

root=Path(__file__).resolve().parents[1]; n=128
heights=(.10,1.30,2.65,4.25);ray_count=24
rays=[]
for i in range(ray_count):
    r=math.sqrt((i+.5)/ray_count);a=i*2.3999632297
    rays.append(Vector((r*math.cos(a),r*math.sin(a),math.sqrt(1-r*r))))

def save(path,rows,size=n,width=None):
    def chunk(tag,data):return struct.pack('>I',len(data))+tag+data+struct.pack('>I',zlib.crc32(tag+data)&0xffffffff)
    hdr=struct.pack('>2I5B',width or size,size,8,6,0,0,0)
    path.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',hdr)+chunk(b'IDAT',zlib.compress(b''.join(b'\x00'+row for row in rows),9))+chunk(b'IEND',b''))

def street_return(tree,stone_faces,at,rays,sun):
    # Gather visible stone below and beside the receiver. A second segment
    # must see the sun. A wall hiding the street cannot act like a fill lamp.
    total=0.
    for ray in rays:
        pos,normal,face,dist=tree.ray_cast(at,ray,10.)
        if pos is None or not stone_faces[face]:continue
        cosine=normal.dot(sun)
        if cosine<=0. or normal.dot(ray)>=0.:continue
        if tree.ray_cast(pos+normal*.025,sun,35.)[0] is None:
            total+=cosine*(1.-(dist/10.)**2)
    return total/len(rays)

morning=json.loads((root/'assets/inks/market-morning.json').read_text())
bearing,alt=map(math.radians,(morning['sun']['bearing_deg'],morning['sun']['elevation_deg']))
sun=Vector((math.sin(bearing)*math.cos(alt),math.cos(bearing)*math.cos(alt),math.sin(alt)))
stone_color=morning['sky']['street_reflectance_linear']
# Stratified over the lower hemisphere; the material's normal supplies the
# receiving form factor at render time. Buildings beyond ten metres use the probe.
down=[]
for i in range(32):
    z=(i+.5)/32.;a=i*2.3999632297;r=math.sqrt(1-z*z)
    down.append(Vector((r*math.cos(a),r*math.sin(a),-z)))

for src,out in [('lanes/orchard.glb','square'),('cistern/cistern.glb','cistern')]:
    if '--town-only' in sys.argv and out!='square':continue
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(root/'assets'/src));bpy.context.view_layer.update()
    verts=[];faces=[];stone_faces=bytearray()
    for obj in bpy.context.scene.objects:
        if obj.type!='MESH':continue
        # Foliage has alpha holes, and the tool box and handwheel can move. Those
        # cast normal live shadows; they do not roof in the entire probe cell.
        lineage=[];p=obj
        while p is not None:lineage.append(p.name.lower());p=p.parent
        if any(any(tok in name for tok in ('canopy_','ripe_plum','wheel_pickup','tonic_lid','tool_lid','sprite_')) for name in lineage):continue
        mesh=obj.data;start=len(verts);verts.extend(obj.matrix_world@v.co for v in mesh.vertices)
        faces.extend(tuple(start+i for i in poly.vertices) for poly in mesh.polygons)
        if out=='square':
            stone=[m is not None and any(s in m.name for s in ('Paving','Setts','Stone','Masonry','Plaster','Lime','Mortar')) for m in mesh.materials]
            stone_faces.extend(bool(stone) and stone[f.material_index] for f in mesh.polygons)
    tree=BVHTree.FromPolygons(verts,faces);rows=[]
    for iz in range(n):
        row=bytearray();bands=[bytearray() for _ in heights];z=(iz+.5)/4.-16.
        for ix in range(n):
            x=(ix+.5)/4.-16.
            for band,h in zip(bands,heights):
                at=Vector((x,-z,h));total=0.
                for ray in rays:
                    dist=tree.ray_cast(at,ray,8.)[3]
                    total+=1. if dist is None else .06+.48*(dist/8.)**2
                sky=round(255*(.14+.86*total/ray_count))
                if out=='square':
                    spill=street_return(tree,stone_faces,at,down,sun)
                    band.append(sky);band.extend(round(255*spill*c) for c in stone_color)
                else:row.append(sky)
        rows.append(b''.join(bands) if out=='square' else bytes(row))
    save(root/'assets/inks'/f'{out}-shelter.png',rows,width=4*n if out=='square' else n)
    print('Sky shelter:',src,len(verts),len(faces),n*n*len(heights)*ray_count,'rays')
    if out=='square':
        # Short-radius hemisphere against the actual bases of buildings, stall
        # legs, trunks, basin, wall and furniture. Tile topology is not sampled.
        size=512;radius=.48;contact_rays=[]
        for i in range(20):
            z=(i+.5)/20.;a=i*2.3999632297;r=math.sqrt(1-z*z)
            contact_rays.append(Vector((r*math.cos(a),r*math.sin(a),z)))
        rows=[];values=[]
        for iz in range(size):
            row=bytearray();z=(iz+.5)*32/size-16
            for ix in range(size):
                x=(ix+.5)*32/size-16;at=Vector((x,-z,.027));total=0.
                for ray in contact_rays:
                    dist=tree.ray_cast(at,ray,radius)[3]
                    if dist is not None:total+=(1-dist/radius)**1.35
                visibility=max(.28,1-.86*total/len(contact_rays));value=round(visibility*255)
                row.extend((value,value,value,255));values.append(value)
            rows.append(bytes(row))
        save(root/'assets/inks/square-contact.png',rows,size)
        (root/'art/lane/contact-bake.json').write_text(json.dumps({
            'source':'assets/lanes/orchard.glb','generator':'tools/bake_sky_shelter.py',
            'map':'assets/inks/square-contact.png','bounds_xz':[-16,16],
            'size':size,'ray_count':len(contact_rays),'ray_radius_m':radius,
            'receiver_height_m':.027,'minimum_visibility':min(values)/255,
            'coordinates':'PNG row 0 is game z=-16, column 0 is game x=-16; linear grayscale',
            'shader':'world_palette.wgsl bindings107/108; floor_style.w marks town receivers; height-faded below .35m',
        },indent=2))
        print('Ground contact:',size*size*len(contact_rays),'rays',min(values),max(values))
