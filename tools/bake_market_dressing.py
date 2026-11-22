"""blender-4.5 -b -P tools/bake_market_dressing.py. Feet, window reveals and pier faces are in map metres."""
import bpy, math, json, random, struct
from pathlib import Path
root=Path(__file__).resolve().parents[1]; art=root/'art/market'; out=root/'assets/market';uvs=json.loads((art/'furnishings-uv.json').read_text())
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
im=bpy.data.images.load(str(art/'furnishings-atlas.png')); im.pack()
mat=bpy.data.materials.new('Painted market leaves and wares');mat.use_nodes=True
bs=next(n for n in mat.node_tree.nodes if n.type=='BSDF_PRINCIPLED');bs.inputs['Roughness'].default_value=.90
s=mat.node_tree.nodes.new('ShaderNodeTexImage');s.image=im;s.interpolation='Closest';mat.node_tree.links.new(s.outputs['Color'],bs.inputs['Base Color']);mat.node_tree.links.new(s.outputs['Alpha'],bs.inputs['Alpha']);mat.surface_render_method='DITHERED';mat.use_backface_culling=False
class Props:
 def __init__(self):self.v=[];self.f=[];self.uv=[]
 def poly(self,pts,uv):
  i=len(self.v);self.v.extend((x,-z,y) for x,y,z in pts);self.f.append(tuple(range(i,i+len(pts))));self.uv+=uv
 def card(self,n,x,y,z,w,tilt=25,yaw=0,flip=False):
  ox,oy,u,v=uvs[n];h=w*v/u;t=math.radians(tilt);a=math.radians(yaw)
  dx,dz=w*.5*math.cos(a),w*.5*math.sin(a);horiz=math.sin(t)*h;hx,hz=horiz*math.sin(a),-horiz*math.cos(a);hy=math.cos(t)*h
  left,right=(ox+.5)/512,(ox+u-.5)/512;top,bottom=1-(oy+.5)/256,1-(oy+v-.5)/256
  if flip:left,right=right,left
  self.poly([(x-dx,y,z-dz),(x+dx,y,z+dz),(x+dx+hx,y+hy,z+dz+hz),(x-dx+hx,y+hy,z-dz+hz)],[(left,bottom),(right,bottom),(right,top),(left,top)])
 def object(self,name):
  mesh=bpy.data.meshes.new(name);mesh.from_pydata(self.v,[],self.f);mesh.update();lm=mesh.uv_layers.new()
  for i,uv in enumerate(self.uv):lm.data[i].uv=uv
  ob=bpy.data.objects.new(name,mesh);bpy.context.collection.objects.link(ob);mesh.materials.append(mat);return ob

def export(ob,name):
 bpy.ops.object.select_all(action='DESELECT');ob.select_set(True);bpy.context.view_layer.objects.active=ob
 path=out/name;bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_texcoords=True,export_materials='EXPORT',export_normals=True,export_animations=False)
 b=path.read_bytes();n=struct.unpack_from('<I',b,12)[0];doc=json.loads(b[20:20+n]);tail=b[20+n:];
 for m in doc['materials']:m['alphaMode']='MASK';m['alphaCutoff']=.50;m['doubleSided']=True
 data=json.dumps(doc,separators=(',',':')).encode();data+=b' '*(-len(data)%4);path.write_bytes(struct.pack('<4sII',b'glTF',2,20+len(data)+len(tail))+struct.pack('<I4s',len(data),b'JSON')+data+tail)

town=Props()
# Wares stand on the actual counter or on solid shopfront cells; flowers stay in the fountain bed.
for args in [(0,-7.78,1.56,2.23,1.05,4),(0,-6.24,1.56,2.23,.98,4),(0,-7.43,1.30,-2.93,.9,5),
 (1,3.0,.31,.98,.78,28),(3,4.98,.30,3.02,.72,27),(1,3.03,.25,3.05,.83,34),(3,5.00,.19,.95,.65,32),
 (4,-8.15,.03,2.03,1.13,24),(6,-3.55,.78,2.78,.34,16),(4,-6.22,.02,6.46,.88,20),(6,6.34,.03,2.29,.50,20),
 (2,-5.68,1.29,1.86,.54,3),(2,-7.90,2.46,-2.69,.54,4),(5,-3.1,1.31,-6.14,.80,0),(5,3.12,1.31,-6.14,.80,0),
 (1,-3.25,.03,-5.80,1.10,28),(3,3.43,.02,-5.92,1.02,30),(2,3.88,2.9,-6.06,.83,6),
 (3,-8.60,.04,5.57,1.16,27),(1,7.6,.02,4.16,.83,29),(1,2,.38,-2.7,.49,30),
 (1,2,.32,-.28,.48,31),(3,8.75,.04,-2.11,.73,34),(0,10.49,1.29,1.30,.98,2),
 (0,-9.60,1.26,1.79,.94,3),(3,-10.03,.04,2.68,.95,21),(4,9.68,.06,.88,1.11,15)]:town.card(*args)
# Low plants break the orchard's straight earth flags. They are ankle-height,
# leaving the crossable ground and the path beneath the cursor legible.
growth=random.Random(604);grid=(root/'assets/maps/town.grid').read_text().splitlines()
for iz,row in enumerate(grid):
 for ix,g in enumerate(row):
  if g!='g':continue
  for i in range(3):
   ox=growth.uniform(-.39,.39);oz=growth.uniform(-.39,.39)
   town.card(1 if i==0 else 7,ix-10+ox,.012,iz-7+oz,growth.uniform(.29,.47),growth.uniform(43,65),growth.uniform(-29,29),i%2)
under=Props()
for args in [(5,-1.0,1.0,-4.32,.68,0),(5,7.03,.84,-6.48,.73,0),
 (7,-3.25,.10,1.34,.65,23),(7,3.06,.10,-3.18,.7,28),(7,-8.50,-.12,-2.39,.53,32),
 (2,-6.70,1.01,-5.43,.48,1),(2,1.20,1.40,-4.45,.45,1),(6,-6.27,.08,.18,.41,20),
 (7,6.85,-.11,1.03,.53,26),(7,4.20,-.06,-5.64,.68,25),(7,-2.75,.02,4.65,.61,20),
 (2,3.07,.59,.49,.44,2),(7,9.09,.02,.23,.48,25),(7,-9.0,-.12,.42,.58,30)]:under.card(*args)
export(town.object('Town flowers and market wares'),'market-trimmings.glb');export(under.object('Cistern banks and hanging standards'),'spring-trimmings.glb')
