#!/usr/bin/env python3
"""Copper Kettle: authored 3D market streets, Blender 4.5, north is game -Z.
Run blender-4.5 --background --python-exit-code 1 --python tools/make_lane.py -- --export-only.
Original authored pixel surfaces + original meshes; no third-party art.
The grid is geometry's authority. All walkable surfaces have top height zero.
"""
import bpy, bmesh, math, random, json, struct
import numpy as np
from pathlib import Path
from mathutils import Vector
import sys
sys.dont_write_bytecode=True
sys.path.insert(0,str(Path(__file__).resolve().parent))
from lane_pixels import surface, worked_material, flower, maps, linear

ROOT=Path(__file__).resolve().parents[1]
ART=ROOT/'art/lane'; OUT=ROOT/'assets/lanes'
ART.mkdir(parents=True,exist_ok=True); OUT.mkdir(parents=True,exist_ok=True)
R=random.Random(19117)
DAYLIGHT=json.loads((ROOT/'assets/inks/market-morning.json').read_text())
bear=math.radians(DAYLIGHT['sun']['bearing_deg']);elev=math.radians(DAYLIGHT['sun']['elevation_deg'])
SUNWARD=Vector((math.sin(bear)*math.cos(elev),math.sin(elev),-math.cos(bear)*math.cos(elev)))
bpy.context.preferences.filepaths.save_version=0
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
for material in list(bpy.data.materials): bpy.data.materials.remove(material)
grid=(ROOT/'assets/maps/town.grid').read_text().splitlines()
assert len(grid)==15 and all(len(row)==21 for row in grid)
def glyph(x,z):
 return grid[z+7][x+10] if -10<=x<=10 and -7<=z<=7 else '#'
def cv(p):return (p[0],-p[2],p[1])

M={}; SOURCE=[]
master=bpy.data.images.load(str(ART/'town_materials_generated.png'))
mw,mh=master.size[:]
# Store the authored swatches in their natural four-by-four layout, in linear
# colour. Four samples per axis give one reference value per 64px brush texel.
studies=np.asarray(master.pixels[:],dtype=np.float32).reshape(mh,mw,4).reshape(4,64,4,4,64,4,4).mean(axis=(2,5))
def image(name,data,linear=False,encode_srgb=False):
 h,w=data.shape[:2]; im=bpy.data.images.new(name,width=w,height=h,alpha=True)
 if linear: im.colorspace_settings.name='Non-Color'
 storage=data
 if encode_srgb:
  # Palette tools author linear radiance; byte colour images store sRGB.
  storage=data.copy();rgb=storage[:,:,:3]
  storage[:,:,:3]=np.where(rgb<=.0031308,rgb*12.92,1.055*np.maximum(rgb,0)**(1/2.4)-.055)
 im.pixels.foreach_set(np.ascontiguousarray(storage,dtype=np.float32).ravel())
 im.filepath_raw=str(ART/(name+'.png'));im.file_format='PNG';im.save()
 return im
def material(name,tile=None,colour=None,rough=.86,metal=0,emission=0,alpha=False,patch=None):
 m=bpy.data.materials.new('Lane_'+name);m.use_nodes=True
 nodes=m.node_tree.nodes;links=m.node_tree.links
 p=next(n for n in nodes if n.type=='BSDF_PRINCIPLED');p.inputs['Roughness'].default_value=rough
 p.inputs['Metallic'].default_value=metal
 if tile is not None or patch is not None:
  if patch is not None:
   source=bpy.data.images.load(str(ART/patch[0]),check_existing=False)
   src=np.asarray(source.pixels[:],dtype=np.float32).reshape(1024,1024,4)
   row,col=divmod(patch[1],2)
   # Retain the original architectural illustration in a quiet 64px vocabulary.
   crop=src[(1-row)*512:(2-row)*512,col*512:(col+1)*512]
   data=crop.reshape(64,8,64,8,4).mean(axis=(1,3))
   data[:,:,:3]=np.round(data[:,:,:3]*20)/20
  else:
   data=surface(name)
   if name in ('Slate','WeatheredSlate','CopperRoof','Wood'):
    study_tile=1 if name in ('Slate','WeatheredSlate','CopperRoof') else tile
    row,col=divmod(study_tile,4)
    data=worked_material(name,data,studies[3-row,:,col,:,:])
  if alpha and name not in ('Leaves','Petals'):
   h,w=data.shape[:2];yy,xx=np.mgrid[0:h,0:w];u=(xx-w/2)/(w/2);v=(yy-h/2)/(h/2)
   data[:,:,3]=((abs(u)*.78+abs(v)<.94)&((u*u+v*v)<.86)).astype(np.float32)
  procedural_colour=name in ('Paving','BackstreetSetts','Stone','Masonry','CanalMasonry','Plaster','WorkshopLime')
  im=image('town_'+name.lower(),data,encode_srgb=procedural_colour);tx=nodes.new('ShaderNodeTexImage');tx.image=im
  # The world shader snaps to a material pixel before this hardware footprint.
  # Cutouts retain their exact painted alpha edge, independent of the ground.
  tx.interpolation='Closest' if alpha else 'Linear';links.new(tx.outputs['Color'],p.inputs['Base Color'])
  relief=4.0 if name in ('Paving','BackstreetSetts') else 2.4 if name in ('Stone','Masonry','CanalMasonry') else .75
  norm,rd=maps(data,rough,relief)
  nt=nodes.new('ShaderNodeTexImage');nt.image=image('town_'+name.lower()+'_normal',norm,True)
  nt.interpolation='Closest';nm=nodes.new('ShaderNodeNormalMap');nm.inputs['Strength'].default_value=.35
  links.new(nt.outputs['Color'],nm.inputs['Color']);links.new(nm.outputs['Normal'],p.inputs['Normal'])
  rt=nodes.new('ShaderNodeTexImage');rt.image=image('town_'+name.lower()+'_roughness',rd,True)
  rt.interpolation='Closest';links.new(rt.outputs['Color'],p.inputs['Roughness'])
  if alpha:
   links.new(tx.outputs['Alpha'],p.inputs['Alpha']);m.surface_render_method='DITHERED'
   m.use_backface_culling=False
  if emission:
   glow=data.copy()
   if name=='Glass':
    yy,xx=np.mgrid[0:glow.shape[0],0:glow.shape[1]]
    core=((xx>20)&(xx<25)&(yy>18)&(yy<43))|((xx>39)&(xx<43)&(yy>27)&(yy<46))
    glow[:,:,:3]*=core[:,:,None]
   et=nodes.new('ShaderNodeTexImage');et.image=image('town_'+name.lower()+'_glow',glow)
   et.interpolation='Closest';links.new(et.outputs['Color'],p.inputs['Emission Color']);p.inputs['Emission Strength'].default_value=emission
  SOURCE.append({'material':m.name,'source_tile':tile,'source_patch':patch,'pixels_per_metre':int(data.shape[1])//(32 if name in ('Paving','BackstreetSetts') else 1)})
 else:
  p.inputs['Base Color'].default_value=(*colour,1)
  if emission:p.inputs['Emission Color'].default_value=(*colour,1);p.inputs['Emission Strength'].default_value=emission
 M[name]=m
 return m
for args in [
 ('Paving',0,.93,0),('Slate',1,.72,0),('Plaster',2,.96,0),('Wood',3,.83,0),
 ('Masonry',4,.94,0),('Crimson',5,.96,0),('Linen',6,.97,0),('Copper',7,.42,.65),
 ('Earth',8,1,0),('Ivy',9,.87,0),('Stone',10,.92,0),('Pottery',11,.78,0),
 ('Glass',12,.22,0),('Banner',13,.96,0),('Ceramic',14,.32,0),('Blossom',15,.85,0)]:
 name,tile,rough,metal=args;material(name,tile,rough=rough,metal=metal,emission=.75 if name=='Glass' else 0)
material('Leaves',9,rough=.89,alpha=True)
material('Petals',15,rough=.9,alpha=True)
material('Mortar',colour=tuple(linear('938780')))
material('BackstreetSetts',0,rough=.91)
material('Moss',colour=tuple(linear('61734c')),rough=.99)
material('LitterLeaf',colour=tuple(linear('8b845c')),rough=.98)
material('Iron',colour=tuple(linear('55777a')),rough=.47,metal=.78)
material('Plum',colour=tuple(linear('995187')),rough=.34)
material('Unripe',colour=tuple(linear('a0ba63')),rough=.48)
material('Apple',colour=tuple(linear('da6c4e')),rough=.49)
material('Gold',colour=tuple(linear('eac781')),rough=.36,metal=.7)
material('Flower',colour=tuple(linear('ffd78b')),rough=.9)
material('WeatheredSlate',1,rough=.78)
material('CopperRoof',7,rough=.56,metal=.42)
material('WorkshopLime',2,rough=.97)
material('CanalMasonry',4,rough=.95)
material('InnSign',patch=('inn_details_generated.png',0),rough=.76)
material('InnCabinet',patch=('inn_details_generated.png',1),rough=.88)
material('InnLattice',patch=('inn_details_generated.png',2),rough=.44,emission=.16)
material('InnPlanterRelief',patch=('inn_details_generated.png',3),rough=.92)
material('WorkshopRack',patch=('workshop_details_generated.png',0),rough=.72)
material('CarvedRelief',patch=('workshop_details_generated.png',1),rough=.94)
material('ValveRelief',patch=('workshop_details_generated.png',2),rough=.44,metal=.55)
material('WheelSign',patch=('workshop_details_generated.png',3),rough=.79)
def sprite_material(name,data,rough=.83,source='tools/lane_pixels.py'):
 m=material(name,colour=(1,1,1),rough=rough)
 nodes=m.node_tree.nodes;links=m.node_tree.links;p=next(n for n in nodes if n.type=='BSDF_PRINCIPLED')
 tx=nodes.new('ShaderNodeTexImage');tx.image=image('town_'+name.lower(),data);tx.interpolation='Closest'
 links.new(tx.outputs['Color'],p.inputs['Base Color']);links.new(tx.outputs['Alpha'],p.inputs['Alpha'])
 normal,rd=maps(data,rough)
 for suffix,pixels,socket in [('normal',normal,'Normal'),('roughness',rd,'Roughness')]:
  node=nodes.new('ShaderNodeTexImage');node.image=image('town_'+name.lower()+'_'+suffix,pixels,True);node.interpolation='Closest'
  if suffix=='normal':
   nm=nodes.new('ShaderNodeNormalMap');nm.inputs['Strength'].default_value=.25
   links.new(node.outputs['Color'],nm.inputs['Color']);links.new(nm.outputs['Normal'],p.inputs[socket])
  else:links.new(node.outputs['Color'],p.inputs[socket])
 m.surface_render_method='DITHERED';m.use_backface_culling=False
 SOURCE.append({'material':m.name,'source':source,'pixels':list(data.shape[:2]),'alpha':'MASK .5; double sided'})
 return m
for name,path in [('CrownFresh','verge-leaves.png'),('CrownBlossom','verge-blossoms.png')]:
 original=bpy.data.images.load(str(ROOT/'art/market'/path),check_existing=False)
 w,h=original.size[:];data=np.asarray(original.pixels[:],dtype=np.float32).reshape(h,w,4)
 lum=data[:,:,:3]@np.array((.2126,.7152,.0722),dtype=np.float32);opaque=data[:,:,3]>.5
 lo,hi=np.percentile(lum[opaque],[4,98])
 tone=np.clip((lum-lo)/max(hi-lo,.01),0,1)
 if name=='CrownFresh':
  ramp=np.array([linear(c) for c in ['355c37','4c733c','698b49','8ba45d','b6c487']])
 else:ramp=np.array([linear(c) for c in ['a15e78','c47c94','e3a3b5','f1c8cb','f7e4d6']])
 data[:,:,:3]=ramp[np.minimum(4,(tone*4.8).astype(int))]
 sprite_material(name,data,source='art/market/'+path+'; calibrated spring-green/pink linear ramps')
sprite_material('FlowerCluster',flower())

class Mesh:
 def __init__(self,name,mat,origin=(0,0,0)):
  self.name=name;self.mat=mat;self.origin=Vector(origin);self.v=[];self.f=[];self.uv=[]
 def face(self,pts,uv=None):
  k=len(self.v);self.v.extend(cv(Vector(p)-self.origin) for p in pts);self.f.append(tuple(range(k,k+len(pts))))
  if uv is None:
   if self.mat in ('Stone','Masonry','CanalMasonry','Plaster','WorkshopLime'):
    # Courses carry around an actual pier, including cut ends of small stones.
    # Starting the texture at (0,0) on every face repeats one pale corner.
    norm=(Vector(pts[1])-Vector(pts[0])).cross(Vector(pts[-1])-Vector(pts[0]))
    axis=max(range(3),key=lambda i:abs(norm[i]));u,v=((2,1),(0,2),(0,1))[axis]
    uv=[(p[u],p[v]) for p in pts]
   else:
    a,b=Vector(pts[0]),Vector(pts[1]);w=(b-a).length
    h=(Vector(pts[-1])-a).length
    uv=[(0,0),(w,0),(w,h),(0,h)] if len(pts)==4 else [(0,0),(w,0),(w*.5,h)]
  self.uv.append(uv)
 def box(self,c,s):
  x,y,z=c;a,b,d=[v*.5 for v in s]
  p=[(x+u*a,y+v*b,z+w*d) for u,v,w in [(-1,-1,-1),(1,-1,-1),(1,-1,1),(-1,-1,1),(-1,1,-1),(1,1,-1),(1,1,1),(-1,1,1)]]
  for ids in [(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]:
   self.face([p[i] for i in reversed(ids)])
 def beam(self,a,b,r1,r2=None,n=8):
  a,b=Vector(a),Vector(b);d=(b-a).normalized();u=d.cross(Vector((0,1,0)))
  if u.length<.01:u=d.cross(Vector((1,0,0)))
  u.normalize();v=d.cross(u);r2=r1 if r2 is None else r2
  rings=[[p+(u*math.cos(i*math.tau/n)+v*math.sin(i*math.tau/n))*r for i in range(n)] for p,r in [(a,r1),(b,r2)]]
  for i in range(n):self.face([rings[0][i],rings[0][(i+1)%n],rings[1][(i+1)%n],rings[1][i]])
  self.face(list(reversed(rings[0])),[(.5,.5)]*n);self.face(rings[1],[(.5,.5)]*n)
 def stone(self,c,s):
  x,y,z=c;a,b,d=[v/2 for v in s]
  shape=[(-.80,-1),(.76,-1),(1,-.68),(1,.72),(.71,1),(-.73,1),(-1,.69),(-1,-.7)]
  lower=[(x+u*a,y-b,z+v*d) for u,v in shape]
  upper=[(x+u*a*.94,y+b,z+v*d*.94) for u,v in shape]
  for i in range(8):self.face([lower[i],upper[i],upper[(i+1)%8],lower[(i+1)%8]])
  self.face(list(reversed(upper)),[((u+1)*s[0]/2,(v+1)*s[2]/2) for u,v in reversed(shape)])
 def finish(self):
  if not self.v:return
  me=bpy.data.meshes.new(self.name);me.from_pydata(self.v,[],self.f);me.materials.append(M[self.mat]);me.update()
  layer=me.uv_layers.new(name='MaterialMetres')
  for poly,uv in zip(me.polygons,self.uv):
   for li,co in zip(poly.loop_indices,uv):layer.data[li].uv=co
  ob=bpy.data.objects.new(self.name,me);bpy.context.collection.objects.link(ob);ob.location=cv(self.origin)
  return ob
def lathe(mesh,c,profile,n=20):
 x,y,z=c
 for (h,r),(h2,r2) in zip(profile,profile[1:]):
  for i in range(n):
   a=i*math.tau/n;b=(i+1)*math.tau/n
   mesh.face([(x+r2*math.cos(a),y+h2,z+r2*math.sin(a)),(x+r2*math.cos(b),y+h2,z+r2*math.sin(b)),(x+r*math.cos(b),y+h,z+r*math.sin(b)),(x+r*math.cos(a),y+h,z+r*math.sin(a))])
def leaf(mesh,p,size,angle,tilt=0):
 p=Vector(p);u=Vector((math.cos(angle),tilt,math.sin(angle))).normalized()*size
 v=Vector((-math.sin(angle)*.7,.72,math.cos(angle)*.7)).normalized()*size
 mesh.face([p-u-v,p+u-v,p+u+v,p-u+v],[(0,0),(1,0),(1,1),(0,1)])
def cluster_card(mesh,c,w,h,yaw=0,tilt=0):
 # Crossed walls and horizontal caps cover north-facing high/eye views.
 c=Vector(c);u=Vector((math.cos(yaw),0,math.sin(yaw)))*w*.5
 v=Vector((-math.sin(yaw)*math.sin(tilt),math.cos(tilt),math.cos(yaw)*math.sin(tilt)))*h*.5
 if mesh.name.startswith('canopy_main'):
  # An actual sun-axis opening, also used by the runtime shaft authoring.
  axis=SUNWARD;opening=Vector((6.05,2.95,-4.55))
  delta=c-opening;near=(delta-axis*delta.dot(axis)).length
  if near<w+h:
   steps=12
   radius=.13+(u.length+v.length)/steps
   for j in range(steps):
    for i in range(steps):
     a=i/steps;b=(i+1)/steps;d=j/steps;e=(j+1)/steps
     middle=c+u*(a+b-1)+v*(d+e-1);q=middle-opening
     if (q-axis*q.dot(axis)).length<radius:continue
     mesh.face([c+u*(2*a-1)+v*(2*d-1),c+u*(2*b-1)+v*(2*d-1),c+u*(2*b-1)+v*(2*e-1),c+u*(2*a-1)+v*(2*e-1)],[(a,d),(b,d),(b,e),(a,e)])
   return
 mesh.face([c-u-v,c+u-v,c+u+v,c-u+v],[(0,0),(1,0),(1,1),(0,1)])

# Level cobbled carpet, precise cell joints. Even the apron is town, not meadow.
ground=Mesh('Lane_Ground','Paving');earth=Mesh('Planted_Verges','Earth');mortar=Mesh('Town_Mortar','Mortar')
apron=Mesh('Backstreet_Setts','BackstreetSetts')
# Split the slab around the genuine below-grade stair throat.
for c,s in [((-13,-.12,-4),(23,.2,41)),((13,-.12,-4),(23,.2,41)),((0,-.12,5),(3,.2,23)),((0,-.12,-17.5),(3,.2,14))]:mortar.box(c,s)
for z in range(-24,17):
 for x in range(-24,25):
  if x in [-1,0,1] and -10<=z<=-5:continue
  ch=glyph(x,z)
  dst=earth if ch=='g' else apron if ch=='#' else ground
  if dst in (ground,apron):
   coords=[((px+16)/32,(16-pz)/32) for px,pz in [(x-.5,z-.5),(x-.5,z+.5),(x+.5,z+.5),(x+.5,z-.5)]]
  else:coords=[(x-.5,z-.5),(x-.5,z+.5),(x+.5,z+.5),(x+.5,z-.5)]
  dst.face([(x-.5,0,z-.5),(x-.5,0,z+.5),(x+.5,0,z+.5),(x+.5,0,z-.5)],coords)
ground.finish();apron.finish();earth.finish();mortar.finish()
# Readable cut-stone crossing and canal-edge course, flush with walking height.
trim=Mesh('Crossing_FlushCutStone','Stone')
for z in range(-6,6):
 for x in [-1.42,1.42]:
  if glyph(round(x),z)!='#':trim.box((x,.003,z),(.085,.008,.965))
for x in range(-8,9):
 for z in [2.47,3.53]:
  if glyph(x,round(z)) not in '#fF':trim.box((x,.003,z),(.965,.008,.065))
trim.finish()

# Shared decorative batches hold the number of draw nodes down.
wood=Mesh('Market_TimberDetails','Wood');stone=Mesh('Town_WeatheredTrim','Stone')
iron=Mesh('Market_Iron','Iron');pots=Mesh('Market_Pottery','Pottery')
ivy=Mesh('Facade_Ivy','Leaves');flowers=Mesh('Flowerbox_Blossoms','Flower')
flower_sprites=Mesh('TownSprite_FlowerBorders','FlowerCluster')
red=Mesh('Awning_Crimson','Crimson');linen=Mesh('Awning_Linen','Linen')
goods=Mesh('Market_Produce','Apple');copper=Mesh('Market_Copper','Copper')
lamps=[]
def flowerbox(x,y,z,w):
 wood.box((x,y,z),(w,.17,.26))
 for dx in [-w*.43,w*.43]:iron.box((x+dx,y-.05,z+.06),(.04,.28,.3))
 for i in range(max(3,int(w*14))):
  xx=x+R.uniform(-w*.46,w*.46)
  leaf(ivy,(xx,y+.15,z),.13,R.random()*math.tau)
  if i%2==0:flowers.beam((xx,y+.18,z+.04),(xx,y+.24,z+.04),.034,.05,6)
 for j in range(max(1,int(w/.35))):
  cluster_card(flower_sprites,(x-w*.33+j*.32,y+.26,z+.04),.35,.40,tilt=.20)
def lantern(x,y,z):
 wood.beam((x,y+.25,z-.15),(x,y+.25,z+.2),.025)
 iron.beam((x,y+.25,z+.18),(x,y+.03,z+.18),.014)
 lamp=Mesh('Lantern_Glass_%02d'%len(lamps),'Glass')
 lamp.box((x,y-.10,z+.18),(.15,.24,.15));lamp.finish()
 for dx in [-.085,.085]:
  for dz in [.095,.265]:iron.box((x+dx,y-.10,z+dz),(.022,.28,.022))
 copper.box((x,y+.05,z+.18),(.22,.05,.22));copper.box((x,y-.25,z+.18),(.21,.045,.21))
 lamps.append({'position':[x,y-.08,z+.30],'range':2.2,'intensity_lumens':65,'colour_srgb':[1,.48,.16]})
def barrel(x,z,r=.25,h=.62):
 lathe(wood,(x,.015,z),[(0,r*.82),(.08,r),(h*.5,r*1.05),(h-.07,r),(h,r*.83)])
 wood.beam((x,h,z),(x,h+.025,z),r*.81,n=16)
 for y in [.08,h*.49,h-.07]:lathe(iron,(x,.015,z),[(y,r*1.025),(y+.034,r*1.025)])
def crate(x,z,w=.55):
 for i in range(5):wood.box((x-w*.4+i*w*.2,.27,z),(w*.18,.5,w))
 for y in [.06,.46]:
  wood.box((x,y,z+w*.5),(w+.04,.075,.055));wood.box((x,y,z-w*.5),(w+.04,.075,.055))
 wood.beam((x-w*.45,.06,z+w*.54),(x+w*.45,.46,z+w*.54),.025,n=4)
def stall(x,z,w=2.4,sidegoods=True):
 # Every stall is on blocked ground. The south edge never overhangs a route.
 for dx in [-w/2,w/2]:
  for dz in [-.43,.43]:wood.box((x+dx,1.05,z+dz),(.07,2.1,.07))
 wood.box((x,.75,z),(w,.12,.85))
 for y in [.17,.39,.6]:wood.box((x,y,z+.44),(w,.14,.055))
 wood.box((x,2.04,z),(w+.16,.08,1.12))
 for j in range(10):
  a=x-w*.5+j*w/10;b=a+w/10
  cloth=Mesh('awning_stall_%s_%s_%02d'%(x,z,j),'Crimson' if j%2==0 else 'Linen',origin=((a+b)*.5,2.25,z-.55))
  cloth.face([(a,1.93,z+.57),(b,1.93,z+.57),(b,2.25,z-.55),(a,2.25,z-.55)])
  cloth.face([(a,1.79,z+.60),(b,1.76,z+.60),(b,1.93,z+.57),(a,1.93,z+.57)])
  cloth.finish()
 for dx in [-w*.31,0,w*.31]:
  wood.box((x+dx,.85,z),(w*.27,.08,.67))
  for j in range(12):
   xx=x+dx+R.uniform(-w*.10,w*.10);zz=z+R.uniform(-.23,.23)
   goods.beam((xx,.91,zz),(xx,.99+R.uniform(0,.06),zz),.055,.045,n=7)
 for dx in [-w*.4,w*.4]:
  iron.beam((x+dx,2.13,z-.05),(x+dx,1.65,z-.05),.012)
  lathe(pots,(x+dx,1.37,z-.05),[(0,.06),(.05,.11),(.18,.10),(.24,.045)])
 if sidegoods:barrel(x-w*.5-.35,z+.15);crate(x+w*.5+.35,z+.12)
 lantern(x+w*.42,1.82,z+.28)

def house(tag,x,z,w,d,h=3.2):
 # Boxes form wall segments around real apertures; there is no plaster behind glass.
 pl=Mesh(tag+'_Plaster','WorkshopLime' if tag=='Weavers' else 'Plaster')
 roof=Mesh(tag+'_Roof',{'Weavers':'WeatheredSlate','EastArcade':'CopperRoof'}.get(tag,'Slate'))
 glass=Mesh(tag+'_RecessedGlass','Glass')
 base=Mesh(tag+'_Foundation','Masonry')
 south=z+d/2;north=z-d/2;left=x-w/2;right=x+w/2
 base.box((x,.19,z),(w,.38,d))
 pl.box((x,(h+.38)/2,north+.07),(w,h-.38,.14))
 # South wall partitioned around two windows, and a genuine inset doorway.
 windows=[(x-w*.28,1.85,.60,.85),(x+w*.28,1.85,.60,.85)]
 holes=[(cx-ww/2,cx+ww/2,cy-hh/2,cy+hh/2) for cx,cy,ww,hh in windows]
 door=(x-.34,x+.34,.38,1.65);holes.append(door)
 cuts=sorted({left,right,*[v for a,b,_,_ in holes for v in [a,b]]})
 for a,b in zip(cuts,cuts[1:]):
  mid=(a+b)/2;hit=next((q for q in holes if q[0]<mid<q[1]),None)
  spans=[(.38,h)] if hit is None else [(.38,hit[2]),(hit[3],h)]
  for lo,hi in spans:
   if hi>lo:pl.box(((a+b)/2,(hi+lo)/2,south-.08),(b-a,hi-lo,.16))
 # Both flank walls have actual cut openings, deep oak reveals and leaded glass.
 for side,xx in [(-1,left),(1,right)]:
  inner=xx-side*.07
  for a,b in [(north,z-.39),(z+.39,south)]:
   pl.box((inner,(h+.38)/2,(a+b)/2),(.14,h-.38,b-a))
  for lo,hi in [(.38,1.45),(2.35,h)]:
   pl.box((inner,(lo+hi)/2,z),(.14,hi-lo,.78))
  glass.box((xx-side*.15,1.9,z),(.035,.86,.74))
  for zz in [z-.43,z+.43]:wood.box((xx+side*.02,1.9,zz),(.13,1.04,.09))
  for yy in [1.39,2.40]:wood.box((xx+side*.02,yy,z),(.13,.09,.96))
  wood.box((xx+side*.05,1.9,z),(.045,.86,.035))
  for yy in [.42,1.18,h-.04]:wood.box((xx+side*.025,yy,z),(.12,.12,d))
  for zz in [north+.06,south-.06]:wood.box((xx+side*.025,(h+.36)/2,zz),(.12,h-.36,.12))
 for cx,cy,ww,hh in windows:
  glass.box((cx,cy,south-.17),(ww-.035,hh-.035,.035))
  for dx in [-ww/2-.04,ww/2+.04]:wood.box((cx+dx,cy,south+.012),(.085,hh+.16,.12))
  for dy in [-hh/2-.04,hh/2+.04]:wood.box((cx,cy+dy,south+.012),(ww+.18,.085,.12))
  for dx in [-.18,0,.18]:wood.box((cx+dx,cy,south-.03),(.026,hh,.055))
  wood.box((cx,cy,south-.015),(ww,.033,.055))
  for dx in [-.46,.46]:wood.box((cx+dx,cy,south+.015),(.16,hh+.08,.07))
  flowerbox(cx,cy-hh/2-.15,south+.15,.83)
 wood.box((x,1.0,south-.11),(.65,1.3,.07))
 for dx in [-.23,0,.23]:wood.box((x+dx,1,south-.06),(.018,1.22,.035))
 iron.beam((x+.2,1,south-.015),(x+.2,1,south+.035),.038,n=8)
 stone.stone((x,.055,south-.005),(.91,.11,.30))
 for xx in [left+.06,x,right-.06]:wood.box((xx,(h+.36)/2,south+.035),(.13,h-.36,.12))
 for yy in [.42,1.18,h-.04]:
  wood.box((x,yy,south+.035),(w+.05,.12,.12));wood.box((right+.02,yy,z),(.12,.12,d))
 for xx,sign in [(left+.12,1),(right-.12,-1)]:
  wood.beam((xx,2.65,south+.055),(xx+sign*w*.25,1.25,south+.055),.052,n=4)
 peak=h+w*({'Weavers':.24,'EastArcade':.18}.get(tag,.34))
 ridge=x-w*.18 if tag=='Weavers' else x
 pl.face([(left,h,south),(right,h,south),(ridge,peak,south)])
 pl.face([(right,h,north),(left,h,north),(ridge,peak,north)])
 wood.beam((x,h,south+.04),(ridge,peak,south+.04),.065,n=4)
 for side in [-1,1]:
  eave=x+side*(w*.5+.13);courses=max(5,round(abs(eave-ridge)/.24))
  for j in range(courses):
   t=(j+.5)/courses;xx=ridge+(eave-ridge)*t
   yy=peak+.06+(h-peak)*(j/courses)
   roof.box((xx,yy-.065,z),(abs(eave-ridge)/courses+.016,.13,d+.28))
  # Projecting copper gutters have an open trough, downspouts and block corbels.
  copper.box((eave,h-.07,z),(.19,.065,d+.34))
  copper.box((eave+side*.085,h-.015,z),(.035,.14,d+.34))
  wood.box((eave-side*.04,h-.15,z),(.15,.16,d+.26))
  for zz in np.arange(north+.15,south,.55):
   for step in range(3):
    wood.box((eave-side*(.06+step*.065),h-.26-step*.065,zz),(.16,.10,.12))
  for zz in [north-.18,south+.18]:
   for j in range(courses):
    xx=ridge+(eave-ridge)*(j+.5)/courses;yy=peak+.02+(h-peak)*(j/courses)
    wood.box((xx,yy-.12,zz),(abs(eave-ridge)/courses+.025,.13,.10))
 for zz in np.arange(north-.18,south+.18,.36):
  roof.box((ridge,peak+.12,min(zz+.17,south+.12)),(.30,.20,.35))
 for xx,side in [(left,-1),(right,1)]:
  copper.box((xx+side*.10,h*.47,north+.13),(.085,h*.90,.085))
  copper.box((xx+side*.06,.10,north+.13),(.19,.15,.14))
 for yy in [.1,.31]:
  for j in range(max(2,int(w/.35))):stone.stone((left+.18+j*.35,yy,south+.02),(.34,.20,.13))
 # Deliberately sparse facade lights; small orange panes, not an emissive wall.
 lantern(right-.27,1.32,south+.04)
 for j in range(3):
  xx=left+.28+j*.18
  for k in range(6):
   leaf(ivy,(xx+math.sin(k)*.06,h-.15-k*.18,south+.10),.12,R.random()*math.tau)
 # Small stone chimneys and caps break the repeated roof silhouette.
 base.box((x-w*.23,h+.85,z-.22),(.38,1.5,.46))
 stone.box((x-w*.23,h+1.63,z-.22),(.52,.12,.59))
 for mesh in [pl,roof,glass,base]:mesh.finish()

# Main inn remains exactly inside its original blocked footprint.
house('CopperKettle',-7.05,.10,2.8,3.1,3.08)
house('Weavers',-6.6,-4.8,3.7,3.45,3.25)
house('Apothecary',-10.3,-.2,3.5,3.8,3.45)
house('Baker',-10.1,-5.0,3.3,4.1,3.1)
house('EastArcade',11.0,-1.1,3.6,4.4,3.1)
house('EastLodging',10.5,-6.0,3.6,4.8,3.9)
house('FountainFlorist',8.6,7.8,3.4,2.4,2.5)
# Receding rooftops close the frame at every zoom. Their ground is all blocked.
for i,(x,z,w,d,h) in enumerate([
 (-14.2,-8,4,4.2,4),(-10,-10.3,4.1,4.2,3.6),(-5.6,-9.5,4.3,4.3,3.8),
 (5.1,-9.5,4.4,4.5,3.8),(10,-11,4.7,4.6,4.6),(14.8,-7,4.6,4.6,4),
 (-14.6,-2,4.2,5,3.7),(15,-1,4.4,5,3.8),(-14,5,5,5,3.4),(14,6,4,4,3.2),
 (-9.4,8.5,3.5,4,2.9),(-5.4,9,3.9,4,2.7),(1,11,5,4,3.2),
 (-17,-14,5,5,5),(-11,-16,5,5,4),(-5,-15.5,5,4,4),(1,-15,5,4,4.2),(7,-16,5,5,5),(14,-15,6,5,4)]):
 house('Backdrop_%02d'%i,x,z,w,d,h)
stall(-10.5,3.6,2.2);stall(-5.7,-6.8,2.3);stall(11.0,2.2,2.0);stall(7.6,8.5,2.6)
stall(-5.05,1.15,.80,sidegoods=False)
stall(-6.15,6.23,2.3,sidegoods=False)
for x,z in [(-9,1.5),(-9,-3),(-8.7,6.1),(9.6,-3.8),(9.8,4.6),(6.4,4.9)]:
 barrel(x,z);crate(x+.53,z-.2,.45)

# Independently painted architectural details are layered over actual joinery.
def panel(name,mat,x,y,z,w,h,west=False):
 ob=Mesh(name,mat)
 pts=[(x,y-h/2,z-w/2),(x,y-h/2,z+w/2),(x,y+h/2,z+w/2),(x,y+h/2,z-w/2)] if west else [(x-w/2,y-h/2,z),(x+w/2,y-h/2,z),(x+w/2,y+h/2,z),(x-w/2,y+h/2,z)]
 ob.face(pts,[(0,0),(1,0),(1,1),(0,1)]);ob.finish()
def shelf(x,y,z,w,mat='InnCabinet'):
 wood.box((x,y,z),(w,.90,.14))
 panel('Recessed_ShopCabinet',mat,x,y,z+.075,w-.06,.84)
 for dy in [-.44,-.12,.19,.46]:wood.box((x,y+dy,z+.17),(w+.05,.05,.28))
 for j in range(6):
  xx=x-w*.39+j*w*.155
  lathe(pots,(xx,y-.08,z+.21),[(0,.044),(.03,.07),(.12,.064),(.16,.03),(.175,.034)],n=10)
  lathe(copper,(xx,y+.23,z+.20),[(0,.042),(.02,.05),(.10,.046),(.13,.032)],n=10)

# Copper Kettle: a covered second-floor wood gallery, dormers, cut lattice panes,
# sunken pantry and a real projecting sign, all behind the interaction row.
gallery=Mesh('Inn_UpperGallery','Wood')
gallery.box((-7.05,1.31,1.93),(2.76,.12,.58))
for xx in [-8.32,-7.47,-6.62,-5.77]:
 gallery.box((xx,1.84,2.18),(.065,1.04,.065))
 gallery.beam((xx,.88,1.66),(xx,1.28,2.13),.038,n=4)
for yy in [1.45,1.92]:gallery.box((-7.05,yy,2.18),(2.70,.055,.055))
for j in range(15):gallery.box((-8.29+j*.177,1.68,2.18),(.028,.43,.028))
gallery.finish()
for j in range(14):
 a=-8.43+j*.197;b=a+.197
 cloth=Mesh('awning_inn_%02d'%j,'Crimson' if j%2==0 else 'Linen',origin=((a+b)*.5,2.92,1.57))
 cloth.face([(a,2.54,2.31),(b,2.54,2.31),(b,2.92,1.57),(a,2.92,1.57)])
 cloth.face([(a,2.39,2.33),(b,2.38,2.33),(b,2.54,2.31),(a,2.54,2.31)])
 cloth.finish()
for xx in [-7.78,-6.32]:
 dormer=Mesh('Inn_DormerJoinery','Wood');dr=Mesh('Inn_DormerSlate','Slate')
 dormer.box((xx,3.72,1.31),(.64,.61,.54))
 panel('Inn_Dormer_Lattice','InnLattice',xx,3.74,1.589,.53,.51)
 for side in [-1,1]:
  for j in range(4):
   dr.box((xx+side*(j+.5)*.10,4.25-j*.075,1.32),(.11,.10,.79))
  copper.box((xx+side*.40,3.99,1.32),(.12,.08,.81))
 dormer.finish();dr.finish()
for xx in [-7.834,-6.266]:
 panel('Inn_CarvedLattice','InnLattice',xx,1.85,1.50,.57,.80)
shelf(-6.12,.84,1.76,.72)
panel('Inn_FloralPlasterRelief','InnPlanterRelief',-8.16,.87,1.755,.49,.74)
wood.box((-5.47,2.51,1.93),(.86,.72,.10))
panel('CopperKettle_EngravedSign','InnSign',-5.47,2.51,1.986,.82,.68)
wood.beam((-5.75,2.97,1.91),(-5.15,2.97,1.91),.04,n=6)
for xx in [-5.72,-5.23]:iron.beam((xx,2.96,1.95),(xx,2.84,1.95),.014)

# Northwest repair shop: a low asymmetric green-grey roof, a deep work hood and
# individual hung tools above the bench. The traverseable g row at z=-2 stays clear.
workroof=Mesh('Workshop_LeanToRoof','WeatheredSlate')
for j in range(5):
 workroof.box((-6.60,2.36+j*.108,-2.58-(j+.5)*.104),(3.64,.11,.114))
wood.box((-6.60,2.24,-2.56),(3.72,.12,.13))
copper.box((-6.60,2.30,-2.50),(3.78,.075,.14))
workroof.finish()
for xx in [-8.3,-4.9]:wood.box((xx,1.15,-2.67),(.075,2.30,.075))
shelf(-7.54,1.02,-3.005,.96,'WorkshopRack')
wood.box((-6.08,.70,-2.82),(1.10,.10,.43))
for xx in [-6.51,-5.65]:wood.box((xx,.34,-2.82),(.07,.68,.07))
for j in range(4):
 xx=-6.47+j*.25
 iron.beam((xx,.77,-2.82),(xx,.88,-2.82),.060,.045,n=10)
 iron.beam((xx,.88,-2.82),(xx+.09,.88,-2.82),.025,n=8)
panel('Workshop_WaterwheelSign','WheelSign',-5.28,1.76,-3.00,.64,.69)
wood.box((-5.28,1.76,-3.06),(.69,.74,.10))

# East shop: verdigris shallow roof and an actual stone arcade on its west side.
arcade=Mesh('EastShop_StoneArcade','CanalMasonry');arcroof=Mesh('EastShop_PentRoof','CopperRoof')
for zz in [-3.12,-1.10,.92]:
 arcade.box((8.78,.80,zz),(.30,1.60,.30))
 stone.box((8.78,.14,zz),(.43,.28,.43));stone.box((8.78,1.63,zz),(.42,.16,.42))
for mid in [-2.11,-.09]:
 for j in range(12):
  a=j*math.pi/12;b=(j+1)*math.pi/12
  for xx in [8.61,9.19]:
   pts=[(xx,1.62+r*math.sin(t),mid+r*math.cos(t)) for r,t in [(.86,a),(.86,b),(1.04,b),(1.04,a)]]
   arcade.face(pts if xx<9 else list(reversed(pts)))
 panel('Arcade_PantryInShadow','InnCabinet',9.185,1.07,mid,1.37,1.35,west=True)
arcroof.face([(8.57,2.64,-3.32),(8.57,2.64,1.12),(9.27,2.92,1.12),(9.27,2.92,-3.32)])
arcade.finish();arcroof.finish()
for xx in [10.10,11.90]:
 panel('EastShop_CarvedSpandrel','CarvedRelief',xx,2.65,1.19,.62,.57)
panel('EastShop_WheelHangingSign','WheelSign',9.05,2.12,1.16,.60,.71)
wood.box((9.05,2.12,1.10),(.65,.76,.10))
lantern(8.82,1.93,.90)

# Breakfast surface at precisely .78, open sky above all interactive cells.
table=Mesh('Breakfast_Table','Wood')
table.box((-4,.715,3),(1.40,.13,.74))
for x in [-4.55,-3.45]:
 for z in [2.75,3.25]:table.box((x,.35,z),(.085,.70,.085))
table.box((-4,.40,3),(1.2,.08,.08));table.finish()
linen.box((-4.43,.787,3),(.27,.014,.71))
# The innkeeper works behind a narrow counter, inside the reserved k cell.
counter=Mesh('Keeper_CookingCounter','Wood')
counter.box((-5,.36,3.25),(.62,.72,.30));counter.box((-5,.745,3.25),(.70,.05,.36))
counter.finish()
lathe(pots,(-5.08,.775,3.25),[(0,.10),(.045,.13),(.11,.14),(.115,.12),(.06,.10)],n=16)
lathe(copper,(-4.02,.785,2.78),[(0,.10),(.05,.14),(.20,.13),(.25,.08),(.28,.08),(.25,.065)])
# Original inn sign is below eave and recessed west of the breakfast sightline.
wood.box((-5.02,2.65,2.12),(.95,.075,.08))
wood.beam((-5.48,2.2,2.12),(-4.56,2.63,2.12),.035,n=6)
for x in [-4.94,-4.62]:iron.beam((x,2.61,2.12),(x,2.31,2.12),.012)
lathe(copper,(-4.78,1.90,2.12),[(0,.12),(.06,.21),(.27,.20),(.34,.12),(.37,.12),(.40,.04)])
copper.beam((-4.62,2.1,2.12),(-4.43,2.25,2.12),.04,.025)

# Low wall remains the exact four blocked x=2 cells.
wall=Mesh('Orchard_Wall','Masonry')
wall.box((2,.29,-1.5),(.36,.58,3.92))
for row in range(3):
 for j in range(8):stone.stone((2,.10+row*.20,-3.23+j*.49),(.43,.19,.47))
for j in range(8):
 leaf(ivy,(2.05,.64,-3.15+j*.46),.12,R.random()*math.tau)
wall.finish()

# Old water gate: arch voussoirs and repairs. Nothing crosses the 4m stair route.
gate=Mesh('OldWaterGate_Masonry','CanalMasonry')
for x in [-3.1,3.1]:
 gate.box((x,1.55,-6.95),(1.75,3.1,1.2))
 for row in range(10):
  for col in range(3):stone.stone((x-.57+col*.55,.15+row*.30,-6.27),(.53,.285,.18))
 for j in range(4):stone.box((x-.65+j*.43,3.28,-6.94),(.28,.38,1.28))
 for yy,ww,dd in [(3.10,1.92,1.34),(3.24,2.08,1.48),(3.36,1.94,1.38)]:
  stone.box((x,yy,-6.95),(ww,.13,dd))
 for side in [-1,1]:
  stone.box((x+side*.67,1.50,-6.25),(.21,2.90,.20))
  for j in range(4):
   stone.box((x+side*.65,.12+j*.16,-6.20),(.30+j*.045,.13,.31+j*.045))
radius=2.22
for i in range(15):
 a=i*math.pi/15;b=(i+1)*math.pi/15
 # Radial arch starts at y1.6, passage clearance max 3.82.
 pts=[(r*math.cos(t),1.6+r*math.sin(t),z) for z in [-7.55,-6.35] for r,t in [(radius,a),(radius,b),(radius+.42,b),(radius+.42,a)]]
 for ids in [(0,1,2,3),(7,6,5,4),(0,4,5,1),(1,5,6,2),(2,6,7,3),(3,7,4,0)]:gate.face([pts[j] for j in ids])
# Stepped archivolt and central keystone are a single course, not repeated
# per voussoir. They leave the original 4m passage entirely open.
for side in [-1,1]:
 for j in range(5):
  xx=side*(1.7-j*.30);yy=1.6+math.sqrt(max(0,2.64**2-xx**2))
  stone.box((xx,yy,-6.27),(.34,.18,.24))
stone.box((0,4.12,-6.20),(.42,.49,.32))
gate.finish()
# Repaired shallow stair landing: flat tread tops keep logical feet at y=0.
# The water gate grows out of a real, thick, arched stone aqueduct, not a frame.
aqueduct=Mesh('North_Aqueduct','CanalMasonry')
for side in [-1,1]:
 for j in range(70):
  xx=side*(2.65+(j+.5)*.15)
  centre=min([side*5,side*8,side*11],key=lambda c:abs(c-xx))
  delta=abs(xx-centre)
  opening=1.70+math.sqrt(max(0,.88**2-delta**2)) if delta<.88 else .42
  if delta<.88:
   aqueduct.box((xx,.21,-7.47),(.15,.42,1.30))
   aqueduct.box((xx,(opening+3.92)/2,-7.47),(.15,3.92-opening,1.30))
  else:aqueduct.box((xx,1.96,-7.47),(.15,3.92,1.30))
 for centre in [side*5,side*8,side*11]:
  for j in range(14):
   a=j*math.pi/14;b=(j+1)*math.pi/14
   stone.face([(centre+r*math.cos(t),1.70+r*math.sin(t),-6.805) for r,t in [(.88,a),(1.045,a),(1.045,b),(.88,b)]])
  panel('Aqueduct_FloralKeystone','CarvedRelief',centre,3.17,-6.79,.54,.53)
aqueduct.box((0,4.21,-7.42),(26.3,.22,1.50))
for zz in [-6.72,-8.10]:
 aqueduct.box((0,4.52,zz),(26.3,.42,.17))
for xx in range(-13,14):
 stone.stone((xx,4.80,-6.72),(.44,.15,.28))
aqueduct.finish()
# An actual deep barrel-vault and dark rear sluice create a readable interior.
vault=Mesh('Gate_DeepBarrelVault','CanalMasonry')
for j in range(24):
 a=j*math.pi/24;b=(j+1)*math.pi/24
 vault.face([(2.22*math.cos(a),1.6+2.22*math.sin(a),-6.36),(2.22*math.cos(b),1.6+2.22*math.sin(b),-6.36),(2.22*math.cos(b),1.6+2.22*math.sin(b),-11),(2.22*math.cos(a),1.6+2.22*math.sin(a),-11)])
for xx in [-2.28,2.28]:vault.box((xx,.81,-8.7),(.16,1.62,4.65))
vault.finish()
rear=Mesh('Gate_ShadowedSluice','Mortar');rear.box((0,1.30,-10.97),(4.38,5.20,.08));rear.finish()
for xx in [-1.7,-1.1,-.55,0,.55,1.1,1.7]:iron.box((xx,1.47,-10.89),(.055,2.94,.045))
# Mortar recesses and chamfered stone ends give the tread rhythm real depth.
for i in range(7):
 z=-4.5-(i+.5)*2/7
 stone.box((0,-.05,z),(3.0,.10,.25))
 stone.box((0,-.012,z+.105),(2.98,.024,.040))
 if z<-5.55:
  for x in [-3.1,2.15]:stone.stone((x,.08,z),(.35,.16,.274))
# Past the walkable map edge the stairs descend physically into the cistern.
for i in range(12):
 stone.box((0,-.1*(i+1)-.05,-6.5-(i+.5)/3),(3.0,.10,.32))
 stone.box((0,-.1*(i+1)-.035,-6.5-i/3),(3.03,.065,.055))
 for xx in [-1.67,1.67]:
  stone.box((xx,.26-.1*i,-6.5-(i+.5)/3),(.28,.42,.32))
stone.box((0,-1.25,-10.72),(3.0,.10,.48))
for x in [-3.1,3.1]:
 banner=Mesh('Gate_Banner_'+str(x),'Banner')
 banner.face([(x-.36,1.45,-6.21),(x+.35,1.36,-6.21),(x+.39,2.9,-6.25),(x-.39,2.9,-6.25)],[(0,0),(1,0),(1,1),(0,1)])
 banner.finish();iron.beam((x-.49,2.94,-6.23),(x+.49,2.94,-6.23),.025)

# Empty fountain. Basin interior remains open and the runtime supplies all water.
fount=Mesh('Fountain_Basin','Masonry');ftrim=Mesh('Fountain_CarvedRim','Stone')
lathe(fount,(4,0,2),[(0,1.35),(.10,1.43),(.22,1.36),(.23,1.16),(.28,1.12),(.52,1.12),(.58,1.27),(.72,1.27),(.72,1.05),(.31,1.05)],n=32)
# Interior basin floor is real geometry at .30m, dark from material and shadow.
fount.beam((4,.285,2),(4,.30,2),1.055,n=32)
for i in range(24):
 a=i*math.tau/24;b=(i+.94)*math.tau/24
 for lo,hi,rr in [(.12,.22,1.38),(.59,.74,1.28)]:
  ftrim.face([(4+rr*math.cos(a),lo,2+rr*math.sin(a)),(4+rr*math.cos(b),lo,2+rr*math.sin(b)),(4+rr*math.cos(b),hi,2+rr*math.sin(b)),(4+rr*math.cos(a),hi,2+rr*math.sin(a))])
lathe(ftrim,(4,.30,2),[(0,.37),(.11,.43),(.18,.29),(.92,.23),(1.02,.37),(1.13,.37),(1.19,.20),(1.46,.17),(1.52,.24),(1.58,.06)],n=16)
# Four bronze waterspouts, inward/outward positions explicitly recorded below.
spouts=[]
for dx,dz in [(0,1),(1,0),(0,-1),(-1,0)]:
 a=(4+dx*.18,1.38,2+dz*.18);b=(4+dx*.46,1.31,2+dz*.46)
 copper.beam(a,b,.075,.052,n=10)
 spouts.append([b[0],b[1]-.025,b[2]])
for i in range(13):
 a=i*math.tau/13
 # Flowers stay inside the blocked 3x3 fountain footprint.
 xx=4+math.cos(a)*1.29;zz=2+math.sin(a)*1.29
 for j in range(3):leaf(ivy,(xx,.27+j*.045,zz),.11,R.random()*math.tau)
 flowers.beam((xx,.36,zz),(xx,.43,zz),.035,.045,n=6)
 cluster_card(flower_sprites,(xx,.43,zz),.33,.37,tilt=.24)
 cluster_card(flower_sprites,(xx,.42,zz),.31,.35,yaw=math.pi/2,tilt=.30)
fount.finish();ftrim.finish()
# Waiting at the pump: oak bucket, tall copper jug, shallow redware bowl.
# The Bevy water plane and the lip share this small authoring manifest.
vessels=json.loads((OUT/'water-vessels.json').read_text())
for i,(x,z,r,h) in enumerate(vessels):
 body=(wood,copper,pots)[i]; rim=(iron,copper,pots)[i]
 profile=[(.02,r*.75),(.09,r*.85),(h*.76,r*1.02),(h-.03,r*.94),(h,r),(h,r*.88),(h-.04,r*.86),(.06,r*.68)]
 lathe(body,(x,0,z),profile,16);body.beam((x,.045,z),(x,.062,z),r*.70,n=16)
 if i==0:
  for y,rr in [(h*.27,.9*r),(h*.79,r)]:lathe(iron,(x,0,z),[(y,rr+.007),(y+.027,rr+.007)],16)
  handle=[(x+math.cos(k*math.pi/12)*r,h-.05+math.sin(k*math.pi/12)*r*.90,z) for k in range(13)]
  for a,b in zip(handle,handle[1:]):iron.beam(a,b,.014,n=5)
 elif i==1:
  for side in (-1,1):
   handle=[(x+side*(r*.89+math.sin(k*math.pi/10)*.09),h*(.56+.34*k/10),z) for k in range(11)]
   for a,b in zip(handle,handle[1:]):copper.beam(a,b,.016,n=6)
 lathe(rim,(x,0,z),[(h-.022,r),(h-.01,r*1.025),(h,r*.99)],16)
# Engraved golden valve sleeves, four bolts and a physically modelled open bore.
gold=Mesh('Fountain_BrassValve','Gold')
panel('Fountain_EngravedValvePlate','ValveRelief',4,1.03,2.255,.38,.38)
for j in range(24):
 a=j*math.tau/24;b=(j+1)*math.tau/24
 gold.face([(4+r*math.cos(t),1.03+r*math.sin(t),z) for r,t,z in [(.09,a,2.29),(.16,a,2.29),(.16,b,2.29),(.09,b,2.29)]])
 gold.face([(4+.09*math.cos(a),1.03+.09*math.sin(a),2.255),(4+.09*math.cos(b),1.03+.09*math.sin(b),2.255),(4+.09*math.cos(b),1.03+.09*math.sin(b),2.30),(4+.09*math.cos(a),1.03+.09*math.sin(a),2.30)])
for dx in [-.14,.14]:
 for dy in [-.14,.14]:gold.beam((4+dx,1.03+dy,2.26),(4+dx,1.03+dy,2.31),.026,n=6)
for dx,dz in [(0,1),(1,0),(0,-1),(-1,0)]:
 gold.beam((4+dx*.23,1.37,2+dz*.23),(4+dx*.36,1.335,2+dz*.36),.087,.076,n=12)
for xx,zz in [(3.08,1.08),(4.92,1.08),(3.08,2.92),(4.92,2.92)]:
 stone.box((xx,.19,zz),(.29,.38,.29))
 for yy,span in [(.07,.38),(.32,.35),(.39,.31)]:
  stone.box((xx,yy,zz),(span,.075,span))
 cluster_card(flower_sprites,(xx,.63,zz),.43,.40,tilt=.14)
 cluster_card(flower_sprites,(xx,.62,zz),.43,.40,yaw=math.pi/2,tilt=.24)
 for j in range(12):
  leaf(ivy,(xx+R.uniform(-.11,.11),.43+R.uniform(0,.12),zz+R.uniform(-.11,.11)),.09,R.random()*math.tau)
  flowers.beam((xx+R.uniform(-.1,.1),.49,zz+R.uniform(-.1,.1)),(xx+R.uniform(-.1,.1),.55,zz+R.uniform(-.1,.1)),.028,.034,n=6)
gold.finish()

def tree(x,z,s,tag,blossom=False,fruit=False):
 tr=Mesh('trunk_'+tag,'Wood',origin=(x,0,z))
 canopy=Mesh('canopy_'+tag,'CrownBlossom' if blossom else 'CrownFresh',origin=(x,1.55*s,z))
 def p(dx,y,dz):return (x+dx*s,y*s,z+dz*s)
 tr.beam(p(0,0,0),p(.12,2.35,0),.21*s,.095*s,n=6)
 # Nine deliberately composed lobes, with separate branches. All foliage lives
 # in the canonical canopy node, so fruit-shake animation keeps its old pivot.
 lobes=[(-.68,2.45,-.32,.98),(.54,2.52,-.42,1.02),(-.76,2.38,.44,.96),
        (.64,2.47,.48,.98),(0,2.34,.72,.98),(-.10,2.88,-.08,1.30),
        (-.41,3.00,.32,.92),(.36,3.08,.23,.95),(.04,3.28,-.12,.90)]
 for k,(dx,y,dz,w) in enumerate(lobes):
  if k<6:tr.beam(p(.08,1.35,0),p(dx,y,dz),.075*s,.022*s,n=6)
  # Lobe shells, not hundreds of intersecting individual leaves: luminous,
  # recognisable pixel clusters at all zooms with modest alpha overdraw.
  cluster_card(canopy,p(dx,y+.13,dz),w*1.22*s,w*1.02*s,yaw=0,tilt=.12)
  cluster_card(canopy,p(dx,y+.13,dz),w*1.22*s,w*1.02*s,yaw=math.pi/2,tilt=-.12)
  cluster_card(canopy,p(dx,y+.38,dz),w*1.17*s,w*1.17*s,tilt=-math.pi/2)
 tr.finish();crown_object=canopy.finish()
 # Preserve the historical random stream and therefore every authored fruit
 # centre, despite replacing the old eight 75-leaf noise clouds.
 for _ in range(8*(1+75*6)):R.random()
 if fruit:
  for k in range(9):
   a=k*2.4;c=p(math.cos(a)*.86,2.10+R.uniform(-.1,.18),math.sin(a)*.82)
   fr=Mesh('ripe_plum_'+tag+'_%02d'%k,'Unripe' if tag=='secondary' else 'Plum',origin=canopy.origin)
   lathe(fr,c,[(-.09,.025),(-.055,.08),(0,.10),(.07,.07),(.095,.02)],n=12)
   obj=fr.finish();obj.parent=crown_object;obj.location=(0,0,0)
tree(5,-4,1,'main',fruit=True);tree(8,-2,.85,'secondary',fruit=True)
tree(7.75,5.1,.93,'fountain_garden')
for i,(x,z,s,bloom) in enumerate([(-10.5,5.5,1.4,True),(10,7.5,1.3,True),(-12,-9,1.5,False),(13,-5,1.3,False),(-15,10,1.8,True),(15,11,1.6,True),(-15,-17,2,False),(14,-19,2,False)]):
 tree(x,z,s,'scenery_%02d'%i,blossom=bloom)
# Flat leaves and seeds have a continuous distribution around their source
# trees. Keep the central walking lane clear except for a few blown strays.
scatter=random.Random(7127); fallen=Mesh('Orchard_Leaf_Litter','LitterLeaf')
for cx,cz,s in [(5,-4,1.5),(8,-2,1.2),(-9.2,4.9,1.7),(7.75,5.1,1.4),(-3.3,-4.2,.78),(4.1,2.7,.75)]:
 for j in range(58):
  x=cx+scatter.gauss(0,s);z=cz+scatter.gauss(0,s*.74)
  if glyph(round(x),round(z)) in '#fFaTkbt>' or (x-4)**2+(z-2)**2<2.2:continue
  a=scatter.uniform(0,math.tau);r=scatter.uniform(.026,.065);y=scatter.uniform(.012,.019)
  ux=r*math.cos(a);uz=r*math.sin(a);vx=-uz*.46;vz=ux*.46
  fallen.face([(x-ux,y,z-uz),(x+vx,y+.015,z+vz),(x+ux,y,z+uz),(x-vx,y,z-vz)],[(0,.5),(.5,1),(1,.5),(.5,0)])
fallen.finish()
# Actual small source-local debris and low moss clumps, outside the clean walk.
moss=Mesh('Contact_Moss_Clusters','Moss');chips=Mesh('Contact_Stone_Chips','Stone')
scraps=Mesh('Market_Wood_Shavings','Wood');paper=Mesh('Market_Paper_Scraps','Linen')
for cx,cz,spread in [(5,-4,.52),(8,-2,.42),(7.75,5.1,.48),(2.20,-2.8,.30),(2.21,-.5,.30),(3.04,2.9,.36),(4.95,1.08,.30),(-5.47,1.6,.28),(-6.90,6.6,.32),(10.1,2.6,.38),(-3.5,-6.1,.34)]:
 for k in range(18):
  a=scatter.random()*math.tau;r=spread*math.sqrt(scatter.random());x=cx+math.cos(a)*r;z=cz+math.sin(a)*r
  if abs(x)<1.8:continue
  size=scatter.uniform(.020,.065)
  if k<12:
   pts=[(x+math.cos(j*math.tau/5)*size,.004+scatter.uniform(0,.006),z+math.sin(j*math.tau/5)*size*.68) for j in range(5)]
   moss.face(list(reversed(pts)))
  elif k<15:chips.stone((x,.012,z),(size,.021,size*.68))
  elif cx<0 or cx>9:
   scraps.box((x,.012,z),(size*1.7,.012,.018))
 if cx<0 or cx>9:
  paper.face([(cx+.14,.012,cz+.16),(cx+.22,.022,cz+.19),(cx+.23,.015,cz+.12),(cx+.15,.013,cz+.10)])
for mesh in [moss,chips,scraps,paper]:mesh.finish()
# Foreground hanging flags stay entirely outside the play rectangle.
for x in [-12,12]:
 iron.beam((x,0,7.8),(x,4,7.8),.035)
 flag=Mesh('Foreground_Flag_'+str(x),'Banner')
 flag.face([(x,2.55,7.8),(x+1.0,2.4,7.85),(x+1.1,3.8,7.9),(x,3.9,7.8)],[(0,0),(1,0),(1,1),(0,1)])
 flag.finish()
for mesh in [wood,stone,iron,pots,ivy,flowers,red,linen,goods,copper,flower_sprites]:mesh.finish()
(OUT/'lights.json').write_text(json.dumps([{'pos':spec['position'],'kind':'sconce'} for spec in lamps],indent=2))
# Static architecture is exported in material batches; animated fruit/crowns keep pivots.
for mat in M.values():
 obs=[ob for ob in bpy.context.scene.objects if ob.type=='MESH' and ob.data.materials and ob.data.materials[0]==mat and not ob.name.startswith(('canopy_','trunk_','awning_','ripe_plum_','Fountain_','Breakfast_','Keeper_','TownSprite_'))]
 if len(obs)<2:continue
 bpy.ops.object.select_all(action='DESELECT')
 for ob in obs:ob.select_set(True)
 bpy.context.view_layer.objects.active=obs[0];bpy.ops.object.join();obs[0].name='TownBatch_'+mat.name

# Only triangles/quads enter tangent export. Keep tree pivots and fruit nodes.
for ob in bpy.context.scene.objects:
 if ob.type!='MESH':continue
 bm=bmesh.new();bm.from_mesh(ob.data)
 faces=[f for f in bm.faces if len(f.verts)>4]
 if faces:bmesh.ops.triangulate(bm,faces=faces)
 bm.to_mesh(ob.data);bm.free();ob.data.update()
bpy.ops.object.select_all(action='DESELECT')
for ob in bpy.context.scene.objects:
 if ob.type=='MESH':ob.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'orchard.glb'),export_format='GLB',use_selection=True,export_yup=True,export_normals=True,export_tangents=True,export_texcoords=True,export_materials='EXPORT',export_cameras=False,export_lights=False,export_extras=True)
# glTF alpha-mask foliage writes depth in the same way as authored clipped leaves.
fp=OUT/'orchard.glb';data=fp.read_bytes();size=struct.unpack_from('<I',data,12)[0]
doc=json.loads(data[20:20+size]);tail=data[20+size:]
for mat in doc['materials']:
 if mat['name'] in ['Lane_Leaves','Lane_Petals','Lane_CrownFresh','Lane_CrownBlossom','Lane_FlowerCluster']:
  mat['alphaMode']='MASK';mat['alphaCutoff']=.5;mat['doubleSided']=True
payload=json.dumps(doc,separators=(',',':')).encode();payload+=b' '*((-len(payload))%4)
fp.write_bytes(struct.pack('<4sII',b'glTF',2,20+len(payload)+len(tail))+struct.pack('<I4s',len(payload),b'JSON')+payload+tail)

# North-facing authoring preview matches the main game's 56 degree pitch.
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True
scene.render.resolution_x=1600;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast'
scene.world.use_nodes=True;bg=next(n for n in scene.world.node_tree.nodes if n.type=='BACKGROUND')
bg.inputs[0].default_value=(*[float(v) for v in linear('c8d7ff')],1);bg.inputs[1].default_value=.48
bpy.ops.object.light_add(type='SUN');sun=bpy.context.object;sun.rotation_euler=Vector(cv(-SUNWARD)).to_track_quat('-Z','Y').to_euler()
sun.data.energy=2.4;sun.data.color=DAYLIGHT['sun']['srgb'];sun.data.angle=.10
for spec in lamps:
 bpy.ops.object.light_add(type='POINT',location=cv(spec['position']))
 bpy.context.object.data.energy=12;bpy.context.object.data.color=(1,.43,.13)
 bpy.context.object.data.shadow_soft_size=.13
bpy.ops.object.camera_add();cam=bpy.context.object;scene.camera=cam
def render(pos,target,lens,name,dof=False):
 cam.location=cv(pos);cam.rotation_euler=(Vector(cv(target))-cam.location).to_track_quat('-Z','Y').to_euler()
 cam.data.type='PERSP';cam.data.lens=lens;cam.data.dof.use_dof=dof
 cam.data.dof.focus_distance=(Vector(cv(target))-cam.location).length;cam.data.dof.aperture_fstop=5.6
 scene.render.filepath=str(ART/name);bpy.ops.render.render(write_still=True)
if '--export-only' not in sys.argv:
 render((0,23.4,15.1),(0,.55,-.4),28,'overhead.png')
 render((0,13.5,11.6),(0,.7,-.4),42,'overview.png')
 render((-2,4.1,8.3),(-1,1.25,-1.0),32,'eye_level.png')
 render((-6.9,3.65,7.8),(-6.9,1.65,1.2),44,'inn_detail.png')
bpy.ops.wm.save_as_mainfile(filepath=str(ART/'orchard.blend'))
report={
 'generator':'Blender 4.5 bpy; tools/make_lane.py',
 'source':'Original stepped meshes and limited-palette motifs in tools/lane_pixels.py, worked with value relief from town_materials_generated.png. The crown lobes come from art/market/spring-verge-source.png via make_verge_sprites.py. Inn/workshop ornaments retain their original illustrations. All source paintings were generated for this project.',
 'revision':'Bright handcrafted chunky fantasy town: cream limestone, caramel timber, airy blue slate, verdigris and select vermilion',
 'coordinates':'glTF Y-up; north -Z; metres; walkable paving top exactly 0; tile boundaries integer +/- .5',
 'grid':'assets/maps/town.grid, 21x15, (-10,-7) first cell; architecture only blocked/outside cells',
 'atlas':'Unique 2048px/32m limestone paintings, 64px/m. Typical bonds .234-.484m wide and .125-.234m high, occasional larger. Close honey/peach/grey tones follow broad mineral beds; cool repairs are restrained local clusters. Chiselled pixels, tiny corner losses, mottled mineral islands and worn centres. Meter divisions belong to actual score lines, not colour/noise grid overlays.',
 'textures':SOURCE,'roughness':'Real non-colour roughness maps; cloth/plaster .96, slate .72, copper .42, iron .47, gold .36, window .22',
 'normal_map':'Non-colour tangent relief: paving 4x gradient, masonry 2.4x, other materials .75x; glTF normalTexture strength .35. Brighter worn centres subtly lower roughness. Metals carry actual metallic factors.',
 'procedural_colour_encoding':'Paving, BackstreetSetts, Stone, Masonry, CanalMasonry, Plaster and WorkshopLime are authored in linear RGB then explicitly encoded to byte sRGB. Their normal/roughness maps remain non-colour data. Existing painting-based material encoding is retained.',
 'materials':[m.name for m in M.values()],
 'fruit_prefix':'ripe_plum_main_','secondary_fruit_prefix':'ripe_plum_secondary_',
 'shake_pivot':'canopy_main branch attachment (5,1.55,-4); other crowns use equivalent branch attachments. Main trunk alone shakes; every crown has independent breeze. Awning stripe pivots sit at attached rear beam, linens and supports remain static.',
 'fountain':{'footprint':[[3,1],[5,3]],'centre':[4,2],'basin_floor_y':.30,'water_surface_y':.50,'water_radius':1.035,'rim_top_y':.74,'spouts':spouts,'empty_at_export':True,'central_pedestal_radius':.43},
 'lights':lamps,
 'interaction_clearance':'Breakfast table (-4,3) top .78; keeper (-5,3) and plum (-3,3) have open overhead. Inn gallery and cloth stop at z2.33, behind interaction row3. Walkable stair-landing tread tops remain y0; real descending flight starts beyond north play boundary z=-6.5. Tree world anchors retained; crown pivots at branch attachment.',
 'previews':[] if '--export-only' in sys.argv else ['overhead.png','overview.png','eye_level.png','inn_detail.png'],
 'comparison':'Viewed the supplied warm Japanese-western plaza reference and actual reference/visual/copper-square-game.webp. Retained north-facing street and exact map/props/anchors; replaced repeating slabs and aggressive noise with unique muted warm stone painting and natural green crown ramps.',
 'art_refinement':'Stepped slate roofs and ridge blocks; open verdigris gutters, downspouts and timber corbels; layered gate cornices and archivolt steps; cream stair nosings and descending cheek blocks; clipped pixel flower borders. Canonical tree crown/fruit nodes and material batching preserved.',
 'foliage':{'method':'Nine painted lobes per tree: two crossed upright cutouts plus cap. Main crown locally subdivided for actual sun aperture; other crowns remain 27 quads. Separate six-sided stems and fruit, alpha-mask .5, double-sided real shadows.','materials':['Lane_CrownFresh','Lane_CrownBlossom','Lane_FlowerCluster'],'sprite_nodes':['canopy_main','canopy_secondary','TownSprite_FlowerBorders'],'source':'art/market/spring-verge-source.png, tools/make_verge_sprites.py and tools/lane_pixels.py; calibrated ramps in make_lane.py'},
 'rebuild':'blender-4.5 -b --python-exit-code 1 --python tools/make_lane.py -- --export-only && blender-4.5 -b --python-exit-code 1 --python tools/bake_sky_shelter.py -- --town-only',
 'canopy_aperture':{'centre':[6.05,2.95,-4.55],'sun_axis':list(SUNWARD),'clear_radius_m':.13,'method':'Conservative physical quad removal through every main crown layer; no fake transparency. Direction from assets/inks/market-morning.json.'},
 'breeze':'Explicit canopy branch pivots and individually anchored awning bands; CPU transforms update the same meshes used for live shadows.',
 'mesh_nodes':len([o for o in scene.objects if o.type=='MESH']),
 'triangles':sum(sum(len(p.vertices)-2 for p in o.data.polygons) for o in scene.objects if o.type=='MESH')
}
(ART/'source.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
print('LANE_DONE',json.dumps({'triangles':report['triangles'],'mesh_nodes':report['mesh_nodes'],'fountain':report['fountain']}))
