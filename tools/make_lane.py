#!/usr/bin/env python3
"""Copper Kettle: authored 3D market streets, Blender 4.5, north is game -Z.
Run blender-4.5 --background --python tools/make_lane.py.
Original generated material sheet + original meshes; no third-party art.
The grid is geometry's authority. All walkable surfaces have top height zero.
"""
import bpy, bmesh, math, random, json, struct
import numpy as np
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
ART=ROOT/'art/lane'; OUT=ROOT/'assets/lanes'
ART.mkdir(parents=True,exist_ok=True); OUT.mkdir(parents=True,exist_ok=True)
R=random.Random(19117)
bpy.context.preferences.filepaths.save_version=0
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
for material in list(bpy.data.materials): bpy.data.materials.remove(material)
grid=(ROOT/'assets/maps/town.grid').read_text().splitlines()
assert len(grid)==15 and all(len(row)==21 for row in grid)
def glyph(x,z):
 return grid[z+7][x+10] if -10<=x<=10 and -7<=z<=7 else '#'
def cv(p):return (p[0],-p[2],p[1])

# The generated sheet is a real UV material source, never a concept-plane.
# Each source square is extracted to an independently repeatable 256px map.
sheet=bpy.data.images.load(str(ART/'town_materials_generated.png'),check_existing=False)
pixels=np.asarray(sheet.pixels[:],dtype=np.float32).reshape(1024,1024,4)
M={}; SOURCE=[]
def image(name,data,linear=False):
 h,w=data.shape[:2]; im=bpy.data.images.new(name,width=w,height=h,alpha=True)
 if linear: im.colorspace_settings.name='Non-Color'
 im.pixels.foreach_set(np.ascontiguousarray(data,dtype=np.float32).ravel())
 im.filepath_raw=str(ART/(name+'.png'));im.file_format='PNG';im.save()
 return im
def material(name,tile=None,colour=None,rough=.86,metal=0,emission=0,alpha=False,patch=None,tint=(1,1,1)):
 m=bpy.data.materials.new('Lane_'+name);m.use_nodes=True
 nodes=m.node_tree.nodes;links=m.node_tree.links
 p=next(n for n in nodes if n.type=='BSDF_PRINCIPLED');p.inputs['Roughness'].default_value=rough
 p.inputs['Metallic'].default_value=metal
 if name=='Paving':
  # The plaza uses one flagstone per actual playable square. Prefilter in
  # linear colour before baking; a near-axis camera can otherwise alias chips
  # and hairline cracks into a noisy checkerboard.
  stone_src=bpy.data.images.load(str(ART/'town-single-flags-source.png'),check_existing=False)
  sw,sh=stone_src.size[:]
  src=np.asarray(stone_src.pixels[:],dtype=np.float32).reshape(sh,sw,4)
  bx=np.linspace(0,sw,257).astype(int);by=np.linspace(0,sh,257).astype(int)
  flags=np.add.reduceat(np.add.reduceat(src,by[:-1],axis=0),bx[:-1],axis=1)
  flags/=np.diff(by)[:,None,None]*np.diff(bx)[None,:,None]
 if tile is not None or patch is not None:
  if name=='Paving':data=flags
  elif patch is not None:
   source=bpy.data.images.load(str(ART/patch[0]),check_existing=False)
   src=np.asarray(source.pixels[:],dtype=np.float32).reshape(1024,1024,4)
   row,col=divmod(patch[1],2);data=src[(1-row)*512:(2-row)*512,col*512:(col+1)*512].copy()
  else:
   row,col=divmod(tile,4);data=pixels[(3-row)*256:(4-row)*256,col*256:(col+1)*256].copy()
  data[:,:,:3]*=np.asarray(tint,dtype=np.float32)
  if name=='Paving':
   # Grey freestone takes its warmth from the late sun; reserve red terracotta for roofs.
   luma=data[:,:,:3]@np.asarray((.2126,.7152,.0722),dtype=np.float32)
   data[:,:,:3]=data[:,:,:3]*.34+luma[:,:,None]*np.asarray((1.04,1.025,.99),dtype=np.float32)*.66
  if alpha:
   yy,xx=np.mgrid[0:256,0:256];u=(xx-128)/128;v=(yy-128)/128
   # Leaf-shaped cutouts, with real generated ivy veins inside each leaf.
   data[:,:,3]=((abs(u)*.78+abs(v)<.94)&((u*u+v*v)<.86)).astype(np.float32)
  im=image('town_'+name.lower(),data);tx=nodes.new('ShaderNodeTexImage');tx.image=im
  tx.interpolation='Closest';links.new(tx.outputs['Color'],p.inputs['Base Color'])
  lum=data[:,:,:3].mean(axis=2);dy,dx=np.gradient(lum)
  nn=np.stack((-dx*3.2,-dy*3.2,np.ones_like(dx)),axis=-1)
  nn/=np.linalg.norm(nn,axis=2,keepdims=True)
  norm=np.ones_like(data);norm[:,:,:3]=nn*.5+.5
  nt=nodes.new('ShaderNodeTexImage');nt.image=image('town_'+name.lower()+'_normal',norm,True)
  nt.interpolation='Closest';nm=nodes.new('ShaderNodeNormalMap');nm.inputs['Strength'].default_value=.45
  links.new(nt.outputs['Color'],nm.inputs['Color']);links.new(nm.outputs['Normal'],p.inputs['Normal'])
  rd=np.ones_like(data);rd[:,:,:3]=np.clip(rough+(lum-lum.mean())*.18,.1,1)[:,:,None]
  rt=nodes.new('ShaderNodeTexImage');rt.image=image('town_'+name.lower()+'_roughness',rd,True)
  links.new(rt.outputs['Color'],p.inputs['Roughness'])
  if alpha:
   links.new(tx.outputs['Alpha'],p.inputs['Alpha']);m.surface_render_method='DITHERED'
   m.use_backface_culling=False
  if emission:
   links.new(tx.outputs['Color'],p.inputs['Emission Color']);p.inputs['Emission Strength'].default_value=emission
  SOURCE.append({'material':m.name,'source_tile':tile,'source_patch':patch,'pixels_per_metre':int(data.shape[1])})
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
material('Mortar',colour=(.075,.079,.061))
material('BackstreetSetts',0,rough=.91)
material('Iron',colour=(.064,.07,.068),rough=.53,metal=.8)
material('Plum',colour=(.24,.055,.19),rough=.4)
material('Unripe',colour=(.23,.34,.08),rough=.48)
material('Apple',colour=(.55,.16,.055),rough=.49)
material('Gold',colour=(.72,.48,.10),rough=.6)
material('Flower',colour=(.96,.65,.25),rough=.9)
material('WeatheredSlate',1,rough=.88,tint=(.68,.85,.64))
material('CopperRoof',7,rough=.68,metal=.35,tint=(.56,.75,.64))
material('WorkshopLime',2,rough=.97,tint=(.65,.73,.61))
material('CanalMasonry',4,rough=.95,tint=(.54,.60,.57))
material('InnSign',patch=('inn_details_generated.png',0),rough=.76)
material('InnCabinet',patch=('inn_details_generated.png',1),rough=.88)
material('InnLattice',patch=('inn_details_generated.png',2),rough=.44,emission=.16)
material('InnPlanterRelief',patch=('inn_details_generated.png',3),rough=.92)
material('WorkshopRack',patch=('workshop_details_generated.png',0),rough=.72)
material('CarvedRelief',patch=('workshop_details_generated.png',1),rough=.94)
material('ValveRelief',patch=('workshop_details_generated.png',2),rough=.44,metal=.55)
material('WheelSign',patch=('workshop_details_generated.png',3),rough=.79)

class Mesh:
 def __init__(self,name,mat,origin=(0,0,0)):
  self.name=name;self.mat=mat;self.origin=Vector(origin);self.v=[];self.f=[];self.uv=[]
 def face(self,pts,uv=None):
  k=len(self.v);self.v.extend(cv(Vector(p)-self.origin) for p in pts);self.f.append(tuple(range(k,k+len(pts))))
  if uv is None:
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
  if dst==ground:
   swatch=((x*73856093)^(z*19349663))%16
   row,col=divmod(swatch,4)
   def uv(u,v):return ((col+(u*.992+.004))*.25,(row+(v*.992+.004))*.25)
   coords=[uv(0,1),uv(0,0),uv(1,0),uv(1,1)]
  else:coords=[(x-.5,z-.5),(x-.5,z+.5),(x+.5,z+.5),(x+.5,z-.5)]
  dst.face([(x-.5,0,z-.5),(x-.5,0,z+.5),(x+.5,0,z+.5),(x+.5,0,z-.5)],coords)
ground.finish();apron.finish();earth.finish();mortar.finish()
# Older masons built the centre walk in a staggered bond. Scored joints
# interrupt some individual flags without ever hiding a whole-cell boundary.
score=Mesh('Repairs_In_The_Processional_Way','Mortar')
for z in range(-4,5):
 for x in range(-3,3):
  if glyph(x,z)=='#' or (3*x+5*z)%7 not in (0,1,3):continue
  off=((x+2*z)%3-1)*.16
  if (x+z)%2:score.box((x+off,.001,z),(.012,.003,.97))
  else:score.box((x,.001,z+off),(.97,.003,.012))
score.finish()
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
  cloth=red if j%2==0 else linen
  cloth.face([(a,1.93,z+.57),(b,1.93,z+.57),(b,2.25,z-.55),(a,2.25,z-.55)])
  cloth.face([(a,1.79,z+.60),(b,1.76,z+.60),(b,1.93,z+.57),(a,1.93,z+.57)])
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
  eave=x+side*(w*.5+.13)
  roofpts=[(ridge,peak+.06,north-.12),(eave,h+.02,north-.12),(eave,h+.02,south+.12),(ridge,peak+.06,south+.12)]
  roof.face(roofpts if side<0 else list(reversed(roofpts)))
  # Slate courses have real lipped edges, not just a coloured roof triangle.
  for j in range(1,max(3,int(w*2.5))):
   t=j/max(3,int(w*2.5));xx=ridge+(eave-ridge)*t;yy=peak+.06+(h-peak)*t
   roof.face([(xx,yy,north-.12),(xx,yy-.034,north-.12),(xx,yy-.034,south+.12),(xx,yy,south+.12)])
  for zz in [north-.14,south+.14]:wood.beam((ridge,peak+.07,zz),(eave,h,zz),.055,n=6)
 wood.beam((ridge,peak+.10,north-.18),(ridge,peak+.10,south+.18),.065,n=8)
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
 a=-8.43+j*.197;b=a+.197;cloth=red if j%2==0 else linen
 cloth.face([(a,2.54,2.31),(b,2.54,2.31),(b,2.92,1.57),(a,2.92,1.57)])
 cloth.face([(a,2.39,2.33),(b,2.38,2.33),(b,2.54,2.31),(a,2.54,2.31)])
for xx in [-7.78,-6.32]:
 dormer=Mesh('Inn_DormerJoinery','Wood');dr=Mesh('Inn_DormerSlate','Slate')
 dormer.box((xx,3.72,1.31),(.64,.61,.54))
 panel('Inn_Dormer_Lattice','InnLattice',xx,3.74,1.589,.53,.51)
 for side in [-1,1]:
  pp=[(xx,4.29,.94),(xx+side*.40,3.99,.94),(xx+side*.40,3.99,1.70),(xx,4.29,1.70)]
  dr.face(pp if side<0 else list(reversed(pp)))
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
workroof.face([(-8.42,2.33,-2.58),(-4.78,2.33,-2.58),(-4.78,2.87,-3.10),(-8.42,2.87,-3.10)])
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
radius=2.22
for i in range(15):
 a=i*math.pi/15;b=(i+1)*math.pi/15
 # Radial arch starts at y1.6, passage clearance max 3.82.
 pts=[(r*math.cos(t),1.6+r*math.sin(t),z) for z in [-7.55,-6.35] for r,t in [(radius,a),(radius,b),(radius+.42,b),(radius+.42,a)]]
 for ids in [(0,1,2,3),(7,6,5,4),(0,4,5,1),(1,5,6,2),(2,6,7,3),(3,7,4,0)]:gate.face([pts[j] for j in ids])
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
 if z<-5.55:
  for x in [-3.1,2.15]:stone.stone((x,.08,z),(.35,.16,.274))
# Past the walkable map edge the stairs descend physically into the cistern.
for i in range(12):
 stone.box((0,-.1*(i+1)-.05,-6.5-(i+.5)/3),(3.0,.10,.32))
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
 for j in range(12):
  leaf(ivy,(xx+R.uniform(-.11,.11),.43+R.uniform(0,.12),zz+R.uniform(-.11,.11)),.09,R.random()*math.tau)
  flowers.beam((xx+R.uniform(-.1,.1),.49,zz+R.uniform(-.1,.1)),(xx+R.uniform(-.1,.1),.55,zz+R.uniform(-.1,.1)),.028,.034,n=6)
gold.finish()

def tree(x,z,s,tag,blossom=False,fruit=False):
 tr=Mesh('trunk_'+tag,'Wood',origin=(x,0,z))
 crown=Mesh('canopy_'+tag,'Petals' if blossom else 'Leaves',origin=(x,0,z))
 def p(dx,y,dz):return (x+dx*s,y*s,z+dz*s)
 tr.beam(p(0,0,0),p(.12,2.35,0),.21*s,.095*s,n=10)
 for k in range(8):
  a=k*math.tau/8;dx=math.cos(a)*.72;dz=math.sin(a)*.7;y=2.4+R.uniform(-.15,.65)
  tr.beam(p(.08,1.35,0),p(dx,y,dz),.075*s,.022*s)
  for j in range(75):
   angle=R.random()*math.tau;r=math.sqrt(R.random())*.66
   xx=dx+math.cos(angle)*r;zz=dz+math.sin(angle)*r
   yy=y+.45*math.sqrt(max(0,1-r*r/.44))+R.uniform(-.12,.12)
   leaf(crown,p(xx,yy,zz),R.uniform(.14,.23)*s,R.random()*math.tau,R.uniform(-.5,.5))
 tr.finish();crown.finish()
 if fruit:
  for k in range(9):
   a=k*2.4;c=p(math.cos(a)*.86,2.10+R.uniform(-.1,.18),math.sin(a)*.82)
   fr=Mesh('ripe_plum_'+tag+'_%02d'%k,'Unripe' if tag=='secondary' else 'Plum')
   lathe(fr,c,[(-.09,.025),(-.055,.08),(0,.10),(.07,.07),(.095,.02)],n=12);fr.finish()
tree(5,-4,1,'main',fruit=True);tree(8,-2,.85,'secondary',fruit=True)
tree(7.75,5.1,.93,'fountain_garden')
for i,(x,z,s,bloom) in enumerate([(-10.5,5.5,1.4,True),(10,7.5,1.3,True),(-12,-9,1.5,False),(13,-5,1.3,False),(-15,10,1.8,True),(15,11,1.6,True),(-15,-17,2,False),(14,-19,2,False)]):
 tree(x,z,s,'scenery_%02d'%i,blossom=bloom)
# Flat leaves and seeds have a continuous distribution around their source
# trees. Keep the central walking lane clear except for a few blown strays.
scatter=random.Random(7127); fallen=Mesh('Orchard_Leaf_Litter','Flower')
for cx,cz,s in [(5,-4,1.5),(8,-2,1.2),(-9.2,4.9,1.7),(7.75,5.1,1.4),(-3.3,-4.2,.78),(4.1,2.7,.75)]:
 for j in range(58):
  x=cx+scatter.gauss(0,s);z=cz+scatter.gauss(0,s*.74)
  if glyph(round(x),round(z)) in '#fFaTkbt>' or (x-4)**2+(z-2)**2<2.2:continue
  a=scatter.uniform(0,math.tau);r=scatter.uniform(.026,.065);y=scatter.uniform(.012,.019)
  ux=r*math.cos(a);uz=r*math.sin(a);vx=-uz*.46;vz=ux*.46
  fallen.face([(x-ux,y,z-uz),(x+vx,y+.015,z+vz),(x+ux,y,z+uz),(x-vx,y,z-vz)],[(0,.5),(.5,1),(1,.5),(.5,0)])
fallen.finish()
# Foreground hanging flags stay entirely outside the play rectangle.
for x in [-12,12]:
 iron.beam((x,0,7.8),(x,4,7.8),.035)
 flag=Mesh('Foreground_Flag_'+str(x),'Banner')
 flag.face([(x,2.55,7.8),(x+1.0,2.4,7.85),(x+1.1,3.8,7.9),(x,3.9,7.8)],[(0,0),(1,0),(1,1),(0,1)])
 flag.finish()
for mesh in [wood,stone,iron,pots,ivy,flowers,red,linen,goods,copper]:mesh.finish()
(OUT/'lights.json').write_text(json.dumps([{'pos':spec['position'],'kind':'sconce'} for spec in lamps],indent=2))
# Static architecture is exported in material batches; animated fruit/crowns keep pivots.
for mat in M.values():
 obs=[ob for ob in bpy.context.scene.objects if ob.type=='MESH' and ob.data.materials and ob.data.materials[0]==mat and not ob.name.startswith(('canopy_','trunk_','ripe_plum_','Fountain_','Breakfast_','Keeper_'))]
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
 if mat['name'] in ['Lane_Leaves','Lane_Petals']:
  mat['alphaMode']='MASK';mat['alphaCutoff']=.5;mat['doubleSided']=True
payload=json.dumps(doc,separators=(',',':')).encode();payload+=b' '*((-len(payload))%4)
fp.write_bytes(struct.pack('<4sII',b'glTF',2,20+len(payload)+len(tail))+struct.pack('<I4s',len(payload),b'JSON')+payload+tail)

# North-facing authoring preview matches the main game's 56 degree pitch.
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True
scene.render.resolution_x=1600;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast'
scene.world.use_nodes=True;bg=next(n for n in scene.world.node_tree.nodes if n.type=='BACKGROUND')
bg.inputs[0].default_value=(.45,.53,.65,1);bg.inputs[1].default_value=.48
bpy.ops.object.light_add(type='SUN');sun=bpy.context.object;sun.rotation_euler=(.70,-.60,-.43)
sun.data.energy=2.4;sun.data.color=(1,.87,.69);sun.data.angle=.10
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
render((0,23.4,15.1),(0,.55,-.4),28,'overhead.png',True)
render((0,13.5,11.6),(0,.7,-.4),42,'overview.png',True)
render((-2,4.1,8.3),(-1,1.25,-1.0),32,'eye_level.png',True)
render((-6.9,3.65,7.8),(-6.9,1.65,1.2),44,'inn_detail.png',True)
bpy.ops.wm.save_as_mainfile(filepath=str(ART/'orchard.blend'))
report={
 'generator':'Blender 4.5 bpy; tools/make_lane.py',
 'source':'Original OpenAI generate_image material sheet plus independently generated inn/workshop 2x2 relief sheets (gpt-image-2-codex), generated for this project; original authored meshes and clipped leaves. No third-party assets.',
 'revision':'Copper Kettle dense Japanese-western fantasy spring market, grid-authoritative original 3D architecture',
 'coordinates':'glTF Y-up; north -Z; metres; walkable paving top exactly 0; tile boundaries integer +/- .5',
 'grid':'assets/maps/town.grid, 21x15, (-10,-7) first cell; architecture only blocked/outside cells',
 'atlas':'Original 4x4 single-slab street study, prefiltered to 64 texels per logical tile. Figure, marker and paving share integer centers; fine setts remain at the gutters. Other parts use the 16 authored material cells and architectural detail sheets.',
 'textures':SOURCE,'roughness':'Per-material generated non-colour maps; cloth/plaster .96, slate .72, copper .42, window .22',
 'normal_map':'Per-material tangent maps from generated detail, non-colour, glTF normalTexture',
 'materials':[m.name for m in M.values()],
 'fruit_prefix':'ripe_plum_main_','secondary_fruit_prefix':'ripe_plum_secondary_',
 'shake_pivot':'canopy_main local pivot (5,0,-4); canopy_secondary (8,0,-2)',
 'fountain':{'footprint':[[3,1],[5,3]],'centre':[4,2],'basin_floor_y':.30,'water_surface_y':.50,'water_radius':1.035,'rim_top_y':.74,'spouts':spouts,'empty_at_export':True,'central_pedestal_radius':.43},
 'lights':lamps,
 'interaction_clearance':'Breakfast table (-4,3) top .78; keeper (-5,3) and plum (-3,3) have open overhead. Inn gallery and cloth stop at z2.33, behind interaction row3. Walkable stair-landing tread tops remain y0; real descending flight starts beyond north play boundary z=-6.5. Main trees retain exact pivots.',
 'previews':['overhead.png (north,56deg, game FOV)','overview.png (north gameplay detail)','eye_level.png (deep gate)','inn_detail.png (layered original storefront)'],
 'comparison':'Viewed captures/plum-morning-j8usqj35/40-morning-at-inn.png, original town.png and reference/visual/copper-square-study.webp. Replaced the meadow/isolated house with contiguous streetfronts, cut-window reveals, generated stone/slate/wood surface detail and striped real-3D market stalls. First rendered crop revealed missing street-facing west windows and excessive narrow FOV; corrected both and added inn-side and foreground stalls.',
 'art_refinement':'Inn: original lattice, engraved name plaque, recessed stocked cabinet, two real dormers and covered oak gallery. Workshop: asymmetric green-slate silhouette, hooded repair bench, independent tool-rack/waterwheel sheet. East shop: low verdigris roof, real stone arcade and carved spandrels. North: 26m thick arched aqueduct, barrel-vault interior and shadowed iron sluice. Fountain: golden bolted valve cover, sleeves, stone-corner flower beds and garden tree shadow.',
 'mesh_nodes':len([o for o in scene.objects if o.type=='MESH']),
 'triangles':sum(sum(len(p.vertices)-2 for p in o.data.polygons) for o in scene.objects if o.type=='MESH')
}
(ART/'source.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
print('LANE_DONE',json.dumps({'triangles':report['triangles'],'mesh_nodes':report['mesh_nodes'],'fountain':report['fountain']}))
