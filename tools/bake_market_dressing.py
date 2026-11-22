"""Export painted dressing after make_lane.py and make_cistern.py.
Each attachment shares its explicit source owner's UV ID and physical bounds.
"""
import bpy, math, json, random, struct, sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]; art=root/'art/market'; out=root/'assets/market';uvs=json.loads((art/'furnishings-uv.json').read_text())
sys.path.insert(0,str(root/'tools'))
from sight_owners import OwnerMap
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
im=bpy.data.images.load(str(art/'furnishings-atlas.png')); im.pack()
aw,ah=im.size[:]
mat=bpy.data.materials.new('Painted market leaves and wares');mat.use_nodes=True
bs=next(n for n in mat.node_tree.nodes if n.type=='BSDF_PRINCIPLED');bs.inputs['Roughness'].default_value=.90
s=mat.node_tree.nodes.new('ShaderNodeTexImage');s.image=im;s.interpolation='Closest';mat.node_tree.links.new(s.outputs['Color'],bs.inputs['Base Color']);mat.node_tree.links.new(s.outputs['Alpha'],bs.inputs['Alpha']);mat.surface_render_method='DITHERED';mat.use_backface_culling=False
class Props:
 def __init__(self,parent_path,start,stop):
  self.v=[];self.f=[];self.uv=[];self.owners=[]
  self.parent_path=parent_path
  self.parents=json.loads(parent_path.read_text())
  self.parent_names={row['name']:row for row in self.parents}
  self.site=OwnerMap(start,stop)
 def poly(self,pts,uv,owner):
  i=len(self.v);self.v.extend((x,-z,y) for x,y,z in pts);self.f.append(tuple(range(i,i+len(pts))));self.uv+=uv;self.owners.append(owner)
 def card(self,n,x,y,z,w,tilt=25,yaw=0,flip=False,owner=None):
  ox,oy,u,v=uvs[n];h=w*v/u;t=math.radians(tilt);a=math.radians(yaw)
  dx,dz=w*.5*math.cos(a),w*.5*math.sin(a);horiz=math.sin(t)*h;hx,hz=horiz*math.sin(a),-horiz*math.cos(a);hy=math.cos(t)*h
  left,right=(ox+.5)/aw,(ox+u-.5)/aw;top,bottom=1-(oy+.5)/ah,1-(oy+v-.5)/ah
  if flip:left,right=right,left
  pts=[(x-dx,y,z-dz),(x+dx,y,z+dz),(x+dx+hx,y+hy,z+dz+hz),(x-dx+hx,y+hy,z-dz+hz)]
  bounds=[[round(min(p[i] for p in pts),5) for i in range(3)],[round(max(p[i] for p in pts),5) for i in range(3)]]
  if isinstance(owner,str):
   row=self.parent_names[owner];owner=row['id']
   if bounds not in row['boxes']:row['boxes'].append(bounds)
  elif owner is None:
   # Source-centred cards in the same patch share one semantic object.
   name=f'{("flowerbox","flowers","hanging_vine","hydrangea","wares","standard","copperware","fern","rubble","bank_stones")[n]}_{x}_{z}'
   owner=self.site.names.get(name)
   if owner is None:owner=self.site.add(name,[round(x),round(z)])
   self.site.box(owner,*bounds)
  elif owner:self.site.box(owner,*bounds)
  self.poly(pts,[(left,bottom),(right,bottom),(right,top),(left,top)],owner)
 def object(self,name):
  mesh=bpy.data.meshes.new(name);mesh.from_pydata(self.v,[],self.f);mesh.update();lm=mesh.uv_layers.new()
  for i,uv in enumerate(self.uv):lm.data[i].uv=uv
  self.site.uv(mesh,self.owners)
  ob=bpy.data.objects.new(name,mesh);bpy.context.collection.objects.link(ob);mesh.materials.append(mat);return ob
 def write(self,name):
  self.site.write(out/name)
  self.parent_path.write_text(json.dumps(self.parents,separators=(',',':'))+'\n')

def export(ob,name):
 bpy.ops.object.select_all(action='DESELECT');ob.select_set(True);bpy.context.view_layer.objects.active=ob
 path=out/name;bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_texcoords=True,export_materials='EXPORT',export_normals=True,export_animations=False)
 b=path.read_bytes();n=struct.unpack_from('<I',b,12)[0];doc=json.loads(b[20:20+n]);tail=b[20+n:];
 for m in doc['materials']:m['alphaMode']='MASK';m['alphaCutoff']=.50;m['doubleSided']=True
 data=json.dumps(doc,separators=(',',':')).encode();data+=b' '*(-len(data)%4);path.write_bytes(struct.pack('<4sII',b'glTF',2,20+len(data)+len(tail))+struct.pack('<I4s',len(data),b'JSON')+data+tail)

town=Props(root/'assets/lanes/sight.json',850,930)
# Wares stand on the actual counter or on solid shopfront cells; flowers stay in the fountain bed.
for args,owner in [
 ((0,-7.78,1.56,2.23,1.05,4),'WestBuilding'),((0,-6.24,1.56,2.23,.98,4),'WestBuilding'),((0,-7.43,1.30,-2.93,.9,5),'Weavers'),
 ((1,3.0,.31,.98,.78,28),'fountain'),((3,4.98,.30,3.02,.72,27),'fountain'),((1,3.03,.25,3.05,.83,34),'fountain'),((3,5.00,.19,.95,.65,32),'fountain'),
 ((4,-8.15,.03,2.03,1.13,24),None),((6,-3.55,.78,2.78,.34,16),'table'),((4,-6.22,.02,6.46,.88,20),'stall_-6.15_6.23'),((6,6.34,.03,2.29,.50,20),None),
 ((2,-5.68,1.29,1.86,.54,3),'WestBuilding'),((2,-7.90,2.46,-2.69,.54,4),'Weavers'),((5,-3.1,1.31,-6.14,.80,0),'gate'),((5,3.12,1.31,-6.14,.80,0),'gate'),
 ((1,-3.25,.03,-5.80,1.10,28),'gate'),((3,3.43,.02,-5.92,1.02,30),'gate'),((2,3.88,2.9,-6.06,.83,6),'gate'),
 ((3,-8.60,.04,5.57,1.16,27),None),((1,7.6,.02,4.16,.83,29),None),((1,2,.38,-2.7,.49,30),'orchard_wall'),
 ((1,2,.32,-.28,.48,31),'orchard_wall'),((3,8.75,.04,-2.11,.73,34),None),((0,10.49,1.29,1.30,.98,2),'EastArcade'),
 ((0,-9.60,1.26,1.79,.94,3),'Apothecary'),((3,-10.03,.04,2.68,.95,21),None),((4,9.68,.06,.88,1.11,15),'EastArcade')]:
 town.card(*args,owner=owner)
# Each orchard cell's original three-card patch is one local semantic object.
# Keep the original card geometry, random sequence, and clean walking lane.
growth=random.Random(604);grid=(root/'assets/maps/town.grid').read_text().splitlines()
for iz,row in enumerate(grid):
 for ix,g in enumerate(row):
  if g!='g':continue
  patch=town.site.add(f'orchard_growth_{ix-10}_{iz-7}',[ix-10,iz-7])
  for i in range(3):
   ox=growth.uniform(-.39,.39);oz=growth.uniform(-.39,.39)
   town.card(1 if i==0 else 7,ix-10+ox,.012,iz-7+oz,growth.uniform(.29,.47),growth.uniform(43,65),growth.uniform(-29,29),i%2,owner=patch)
for x,z,w in [(-3.38,-4.93,.66),(-4.71,-2.28,.69),(3.20,-5.35,.46),(7.64,-2.03,.76),(4.43,4.18,.59),(-8.13,4.76,.57),(6.83,4.29,.64)]:
 town.card(8,x,.018,z,w,27)
under=Props(root/'assets/cistern/sight.json',930,1024)
for args,owner in [
 ((5,-1.0,1.0,-4.32,.68,0),'column_-1_-5'),((5,7.03,.84,-6.48,.73,0),'column_7_-7'),
 ((7,-3.25,.10,1.34,.65,23),'wall_-3_1'),((7,3.06,.10,-3.18,.7,28),'wall_3_-3'),((7,-8.50,-.12,-2.39,.53,32),None),
 ((2,-6.70,1.01,-5.43,.48,1),'wall_-7_-6'),((2,1.20,1.40,-4.45,.45,1),'column_1_-5'),((6,-6.27,.08,.18,.41,20),'tool'),
 ((7,6.85,-.11,1.03,.53,26),None),((7,4.20,-.06,-5.64,.68,25),None),((7,-2.75,.02,4.65,.61,20),'wall_-3_5'),
 ((2,3.07,.59,.49,.44,2),'wall_3_0'),((7,9.09,.02,.23,.48,25),'wall_9_0'),((7,-9.0,-.12,.42,.58,30),None)]:
 under.card(*args,owner=owner)
for (x,y,z,w),owner in [
 ((-8.53,-.12,1.89,.84),None),((-8.50,-.12,-3.04,.60),None),((4.62,-.13,-5.69,.64),None),
 ((5.82,-.13,1.08,.51),None),((-3.21,.04,.89,.53),'wall_-3_1'),((.81,.03,-4.65,.50),'wall_1_-5'),((8.72,.05,-.02,.74),'wall_9_0')]:
 under.card(9,x,y,z,w,25,owner=owner)
export(town.object('Town flowers and market wares'),'market-trimmings.glb')
export(under.object('Cistern banks and hanging standards'),'spring-trimmings.glb')
town.write('town-sight.json');under.write('cistern-sight.json')
