#!/usr/bin/env python3
"""Original Wanderstead scene. Run: blender-4.5 --background --python tools/make_lane.py
All geometry and pixel textures are original procedural artwork; no third-party assets.
World coordinates passed below are game x,y,z (north=-z); Blender uses x,-z,y.
"""
import bpy, math, random, json
import numpy as np
from mathutils import Vector
from pathlib import Path
from array import array
R=random.Random(19117)
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/lanes'; ART=ROOT/'art/lane'
OUT.mkdir(parents=True,exist_ok=True); ART.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
for d in list(bpy.data.materials): bpy.data.materials.remove(d)
# A single 4x4 hand-authored pixel atlas shared by all surfaces. 128px tiles.
S=512; T=128
pal=[(92,115,57),(160,132,86),(157,148,118),(214,194,145),(98,65,39),(71,106,105),(118,150,62),(149,56,123),(166,92,43),(228,218,179),(195,166,120),(137,141,117),(104,77,43),(60,90,64),(224,196,110),(83,107,111)]
pix=array('f',[0.0])*(S*S*4); rough=array('f',[0.0])*(S*S*4)
def hash2(x,y,k=0): return ((x*73856093 ^ y*19349663 ^ k*83492791)&255)/255
for y in range(S):
 for x in range(S):
  t=(y//T)*4+x//T; u=x%T; v=y%T; base=pal[t]; a=1.; n=hash2(u//2,v//2,t)-.5; q=0
  if t in (0,13):
   q=(hash2(u//7,v//5,t)-.5)*18+n*13
   if hash2(u//3,v//6,t)> .84: q+=13
  elif t==1: q=(hash2(u//8,v//9)-.5)*23+n*10
  elif t in (2,11):
   q=(hash2(u//15,v//12,t)-.5)*25+n*7
   if abs(math.sin(u*.07+math.sin(v*.09))+.55*math.sin(v*.13))<.022:q-=16
  elif t==3:q=(hash2(u//12,v//13)-.5)*10+n*8
  elif t in (4,12):
   q=math.sin(u*.23+math.sin(v*.09)*1.7)*9+math.sin(u*.77+v*.018)*5+n*8
   if u%41<2:q-=24
  elif t==5:
   row=v//24; seam=v%24<3 or (u+(row%2)*15)%32<2
   q=-35 if seam else (v%24)*.6+(hash2((u+(row%2)*15)//32,row)-.5)*23+n*5
  elif t==6:
   # Sprig silhouettes with a vein, irregular spacing, transparent daylight gaps.
   a=0.; q=n*12
   for cx,cy,ang,sz in [(64,63,0,30),(33,42,-.8,23),(89,37,.75,24),(35,83,.9,24),(90,82,-.7,25),(65,106,0,22)]:
    dx=u-cx;dy=v-cy; xx=dx*math.cos(ang)-dy*math.sin(ang); yy=dx*math.sin(ang)+dy*math.cos(ang)
    if abs(xx)/(sz*.55)+abs(yy)/sz<1:
     a=1.;q+=12 if xx<0 else -7
     if abs(xx)<1.5:q+=17
   if abs(u-64)<1.5 and 24<v<110:a=1.;q=-13
  elif t==7: q=n*5+math.cos((u-35)*.05)*13+(v/128)*8
  elif t==8:q=n*8+(hash2(u//10,v//12)-.5)*14
  elif t==9:q=n*4; q-=17 if v%32<3 else 0
  elif t==10:q=n*4 + (18 if u%48<16 else -4)
  elif t==14:q=n*9
  else:q=n*7
  i=(y*S+x)*4
  for c in range(3):pix[i+c]=max(0,min(1,(base[c]+q)/255))
  pix[i+3]=a
  rv=.9 if t not in (7,8,9) else (.48 if t==8 else .58)
  rough[i:i+4]=array('f',[rv,rv,rv,1])
def save_image(name,data,noncolor=False):
 im=bpy.data.images.new(name,width=S,height=S,alpha=True)
 if noncolor:im.colorspace_settings.name='Non-Color'
 im.pixels.foreach_set(data);im.filepath_raw=str(ART/(name+'.png'));im.file_format='PNG';im.save();return im
atlas=save_image('lane_atlas',pix);ra=save_image('lane_roughness',rough,noncolor=True)
# Foliage receives authored light/shade colour groups in one alpha atlas. These
# groups survive the game's stepped lighting and keep each crown legible overhead.
leaf_colours=[(30,62,43),(40,78,44),(51,91,44),(66,107,47),(86,121,49),(103,135,53),(91,127,54),(126,150,64),(151,169,77),(170,182,91),(56,94,67),(77,113,75),(108,133,80),(137,156,84),(64,102,50),(116,141,57)]
lp=np.ones((S,S,4),dtype=np.float32);original=np.asarray(pix,dtype=np.float32).reshape(S,S,4)[128:256,256:384]
detail=(original[:,:,:3].mean(axis=2)-original[:,:,:3].mean())*.65
for ti,colour in enumerate(leaf_colours):
 yy=(ti//4)*T;xx=(ti%4)*T
 lp[yy:yy+T,xx:xx+T,:3]=np.clip(np.array(colour)/255+detail[:,:,None],0,1)
 lp[yy:yy+T,xx:xx+T,3]=original[:,:,3]
leaf_atlas=save_image('lane_foliage',lp.ravel())
# Reusable tangent normal atlas, derived from the authored grain/pits and saved
# as non-colour data. NormalMap is exported as standard glTF, not Blender nodes.
height=np.asarray(pix,dtype=np.float32).reshape(S,S,4)[:,:,:3].mean(axis=2)
gy,gx=np.gradient(height);nn=np.stack((-gx*6,-gy*6,np.ones_like(gx)),axis=-1);nn/=np.linalg.norm(nn,axis=2,keepdims=True)
nr=np.ones((S,S,4),dtype=np.float32);nr[:,:,:3]=nn*.5+.5
normal=bpy.data.images.new('lane_normal',width=S,height=S,alpha=False);normal.colorspace_settings.name='Non-Color';normal.pixels.foreach_set(nr.ravel());normal.filepath_raw=str(ART/'lane_normal.png');normal.file_format='PNG';normal.save()
M={};tiles={}
for name,tile in [('Terrain',0),('Stone',11),('Plaster',3),('Wood',4),('Slate',5),('Leaves',6),('Plum',7),('Copper',8),('Ceramic',9),('Linen',10)]:
 m=bpy.data.materials.new('Lane_'+name);m.use_nodes=True;m.node_tree.nodes.clear();p=m.node_tree.nodes.new('ShaderNodeBsdfPrincipled');op=m.node_tree.nodes.new('ShaderNodeOutputMaterial');m.node_tree.links.new(p.outputs['BSDF'],op.inputs['Surface']);p.inputs['Roughness'].default_value=.87
 tx=m.node_tree.nodes.new('ShaderNodeTexImage');tx.image=leaf_atlas if name=='Leaves' else atlas;tx.interpolation='Closest';m.node_tree.links.new(tx.outputs['Color'],p.inputs['Base Color'])
 if name!='Leaves':
  rt=m.node_tree.nodes.new('ShaderNodeTexImage');rt.image=ra;rt.interpolation='Closest';m.node_tree.links.new(rt.outputs['Color'],p.inputs['Roughness'])
 if name not in ('Terrain','Leaves','Plum','Ceramic'):
  nt=m.node_tree.nodes.new('ShaderNodeTexImage');nt.image=normal;nt.interpolation='Closest';nm=m.node_tree.nodes.new('ShaderNodeNormalMap');nm.inputs['Strength'].default_value=.48;m.node_tree.links.new(nt.outputs['Color'],nm.inputs['Color']);m.node_tree.links.new(nm.outputs['Normal'],p.inputs['Normal'])
 if name=='Copper':p.inputs['Metallic'].default_value=.72
 if name=='Leaves':
  m.surface_render_method='DITHERED';m.use_transparency_overlap=False;m.node_tree.links.new(tx.outputs['Alpha'],p.inputs['Alpha']);m.use_backface_culling=False
 M[name]=m;tiles[name]=tile
# Collection mesh builder: joins by role/material rather than creating thousands of nodes.
def cv(p):return (p[0],-p[2],p[1])
class Mesh:
 def __init__(self,name,mat,tile=None,origin=(0,0,0)):
  self.name=name;self.mat=mat;self.tile=tiles[mat] if tile is None else tile;self.origin=Vector(origin);self.v=[];self.f=[];self.uv=[]
 def face(self,pts,uv=None,tile=None):
  k=len(self.v);self.v += [cv(Vector(p)-self.origin) for p in pts];self.f.append(tuple(range(k,k+len(pts))))
  if uv is None:uv=[(0,0),(1,0),(1,1),(0,1)] if len(pts)==4 else [(0,0),(1,0),(.5,1)]
  ti=self.tile if tile is None else tile
  self.uv.append([((ti%4+(u*.94+.03))/4,(ti//4+(v*.94+.03))/4) for u,v in uv])
 def box(self,c,s,rot=0,tile=None):
  cx,cy,cz=c;xx,yy,zz=[v*.5 for v in s];p=[]
  for x,y,z in [(-xx,-yy,-zz),(xx,-yy,-zz),(xx,-yy,zz),(-xx,-yy,zz),(-xx,yy,-zz),(xx,yy,-zz),(xx,yy,zz),(-xx,yy,zz)]:
   p.append((cx+x*math.cos(rot)+z*math.sin(rot),cy+y,cz-x*math.sin(rot)+z*math.cos(rot)))
  for ids in [(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]:self.face([p[i] for i in reversed(ids)],tile=tile)
 def beam(self,a,b,r1,r2=None,n=8):
  a=Vector(a);b=Vector(b);d=(b-a).normalized();u=d.cross(Vector((0,1,0)))
  if u.length<.01:u=d.cross(Vector((1,0,0)))
  u.normalize();v=d.cross(u);rr=r1 if r2 is None else r2
  rings=[[p+(u*math.cos(i*math.tau/n)+v*math.sin(i*math.tau/n))*r for i in range(n)] for p,r in [(a,r1),(b,rr)]]
  for i in range(n):self.face([rings[0][i],rings[0][(i+1)%n],rings[1][(i+1)%n],rings[1][i]])
  self.face(list(reversed(rings[0])),uv=[(.5+.45*math.cos(i*math.tau/n),.5+.45*math.sin(i*math.tau/n)) for i in reversed(range(n))]);self.face(rings[1],uv=[(.5+.45*math.cos(i*math.tau/n),.5+.45*math.sin(i*math.tau/n)) for i in range(n)])
 def stone(self,c,s,tile=None):
  cx,cy,cz=c;xx,yy,zz=[x/2 for x in s];shape=[(-.75,-1),(.72,-1),(1,-.63),(1,.67),(.67,1),(-.7,1),(-1,.65),(-1,-.65)]
  lower=[(cx+x*xx,cy-yy*.8,cz+z*zz) for x,z in shape];upper=[(cx+x*xx*.88,cy+yy*.72,cz+z*zz*.88) for x,z in shape]
  for i in range(8):self.face([upper[i],upper[(i+1)%8],lower[(i+1)%8],lower[i]],tile=tile)
  self.face(list(reversed(upper)),uv=[((x+1)/2,(z+1)/2) for x,z in reversed(shape)],tile=tile)
 def finish(self):
  if not self.v:return None
  me=bpy.data.meshes.new(self.name);me.from_pydata(self.v,[],self.f);me.materials.append(M[self.mat]);me.update();uv=me.uv_layers.new(name='PixelAtlas')
  for poly,coords in zip(me.polygons,self.uv):
   for li,co in zip(poly.loop_indices,coords):uv.data[li].uv=co
  ob=bpy.data.objects.new(self.name,me);bpy.context.collection.objects.link(ob);ob.location=cv(self.origin);return ob
# Pixel-painted terrain field. Shared world UVs eliminate square tile shoulders;
# low-frequency meadow growth, pebble flecks and wheel wear remain real texture.
W,H=2048,1792
zz,xx=np.mgrid[0:H,0:W];wx=xx/W*80.5-40.25;wz=zz/H*68.5-44.25
noise=((xx.astype(np.int64)*73856093 ^ zz.astype(np.int64)*19349663)&255)/255.
coarse=(np.sin(wx*1.17+wz*.67)+np.sin(wx*.41-wz*1.31)+np.sin(wx*3.1+wz*2.8))/3
center=-.7+np.sin(wz*.13)*.6
shoulder=1.48+.12*np.sin(wz*2.6)+.07*np.sin(wz*8)+.075*coarse
distance=np.abs(wx-center)-shoulder
# Overlapping rounded areas of wear join door, keeper and road organically.
court=np.full_like(wx,100.)
for cx,cz,rx,rz in [(-7.05,2.65,1.7,1.13),(-5.3,3.05,1.6,1.05),(-3.45,3.22,1.55,.88),(-1.9,3.1,1.5,.67)]:
 d=(np.sqrt(((wx-cx)/rx)**2+((wz-cz)/rz)**2)-1)*min(rx,rz)
 court=np.minimum(court,d)
distance=np.minimum(distance,court)+.085*coarse+.04*(noise-.5)
mix=np.clip(.5-distance*2.8,0,1)
# The orchard footpath bends toward the southern wall mouth and disappears into
# grass in irregular worn patches; there is no rectangle or hard end-cap.
foot_center=1.1+.42*np.sin(wx*.65)+.035*wx
foot_dist=np.abs(wz-foot_center)-(.27+.06*np.sin(wx*2.3))
foot_mix=np.clip(.5-(foot_dist+.06*coarse)*2.6,0,1)*np.clip((10-wx)/3.6,0,1)*np.clip(wx/.8,0,1)
mix=np.maximum(mix,foot_mix)[...,None]
green=np.stack([.29+.026*coarse,.385+.031*coarse,.16+.021*coarse],axis=-1)
soil=np.stack([.54+.022*coarse,.424+.02*coarse,.278+.018*coarse],axis=-1)
fine=(noise-.5)*.028
rgba=np.ones((H,W,4),dtype=np.float32)
rgba[:,:,:3]=green*(1-mix)+soil*mix+fine[...,None]
# Subtle wheel grooves in the dirt, not sharp raised rails.
rut=np.exp(-((np.abs(wx-center)-.77)/.073)**2)*.045
rgba[:,:,:3]-=rut[...,None]*mix
# Grass tufts and scattered pale grit, grouped rather than full-screen noise.
tuft=((xx//2*31 ^ zz//5*71)&127)>113
rgba[:,:,:3]+=tuft[...,None]*(1-mix)*np.array([.025,.036,.008])
grit=((xx//2*73 ^ zz//2*41)&255)>246
rgba[:,:,:3]+=grit[...,None]*mix*np.array([.065,.063,.045])
im=bpy.data.images.new('lane_terrain',width=W,height=H,alpha=False);im.pixels.foreach_set(rgba.ravel());im.filepath_raw=str(ART/'lane_terrain.png');im.file_format='PNG';im.save()
earth=bpy.data.materials.new('Lane_Earth');earth.use_nodes=True;ep=next(n for n in earth.node_tree.nodes if n.type=='BSDF_PRINCIPLED');ep.inputs['Roughness'].default_value=.95
et=earth.node_tree.nodes.new('ShaderNodeTexImage');et.image=im;et.interpolation='Closest';earth.node_tree.links.new(et.outputs['Color'],ep.inputs['Base Color'])
del rgba,wx,wz,xx,zz,green,soil,mix,noise
# Ground: small irregular patches with explicit atlas UV blending between grass, worn soil and cart tracks.
g=Mesh('Lane_Ground','Terrain');stones=Mesh('Lane_Stones','Stone')
for iz in range(-88,49):
 z=iz*.5
 for ix in range(-80,81):
  x=ix*.5;center=-.7+math.sin(z*.13)*.6;dist=abs(x-center);edge=1.5+.23*math.sin(z*1.9)+R.uniform(-.18,.18)
  road=dist<edge;courtyard=(-8.5<x<-3.2 and 1.7<z<4.9);orchardpath=(1<z<2.0 and 0<x<10)
  tile=1 if road or courtyard or orchardpath else (13 if z<-16 else 0)
  # A continuous level surface: no open micro-step seams against the apron.
  h=0
  g.face([(x-.25,h,z-.25),(x-.25,h,z+.25),(x+.25,h,z+.25),(x+.25,h,z-.25)],tile=tile)
  if road and R.random()<.17 and -17<z<12:
   stones.stone((x+R.uniform(-.16,.16),.015,z+R.uniform(-.16,.16)),(R.uniform(.18,.43),.035,R.uniform(.17,.36)),tile=2)
ground_object=g.finish();ground_object.data.materials.clear();ground_object.data.materials.append(earth)
for loop in ground_object.data.loops:
 co=ground_object.data.vertices[loop.vertex_index].co
 ground_object.data.uv_layers.active.data[loop.index].uv=((co.x+40.25)/80.5,(-co.y+44.25)/68.5)
# Low mortared wall at exactly x=2, z=-3..0, both ends open.
wall=Mesh('Orchard_Wall','Stone');moss=Mesh('Wall_Moss','Terrain')
wall.box((2,.345,-1.5),(.32,.69,3.86))
for row in range(3):
 for i in range(9):
  z=-3.43+i*.43+(row%2)*.17
  if z>.4:continue
  wall.stone((2,row*.24+.12,z),(.43+R.uniform(-.035,.035),.25,.45),tile=11)
for z in [-3.45,.43]:wall.stone((2,.40,z),(.56,1.0,.48))
for i in range(35):
 z=R.uniform(-3.5,.45);moss.stone((2+R.uniform(-.12,.12),.725,z),(.16,.025,.18),tile=0)
wall.finish();moss.finish()
# Inn: thick sill, lime plaster, exposed oak frame, individual staggered slate courses.
pl=Mesh('Inn_Plaster','Plaster');wood=Mesh('Inn_Timber','Wood');roof=Mesh('Inn_Roof','Slate');glass=Mesh('Inn_WindowGlass','Plaster',15);base=Mesh('Inn_Foundation','Stone')
# Footprint inside blocked cells only, overhang kept away from adjacent east/table cells.
base.box((-6.5,.22,.5),(3.8,.44,3.8));pl.box((-6.5,1.94,.5),(3.72,3.05,3.72))
for row in range(2):
 for k in range(9):
  base.stone((-8.18+k*.42,.105+row*.22,2.42),(.42,.22,.16),tile=11)
  base.stone((-4.59,.105+row*.22,-1.20+k*.42),(.16,.22,.42),tile=11)
# South gable face is actual triangular plaster rather than giant cuboid.
pl.face([(-8.36,3.46,2.36),(-4.64,3.46,2.36),(-6.5,5.12,2.36)])
pl.face([(-4.64,3.46,-1.36),(-8.36,3.46,-1.36),(-6.5,5.12,-1.36)])
for x in [-8.37,-6.5,-4.63]:wood.box((x,1.95,2.395),(.16,3.17,.13))
for z in [-1.4,.5,2.39]:wood.box((-4.605,1.95,z),(.14,3.18,.16))
for y in [.49,1.74,3.4]:
 wood.box((-6.5,y,2.4),(3.9,.15,.15));wood.box((-4.6,y,.5),(.15,.15,3.9))
for x in [-8.3,-4.7]:wood.beam((x,3.4,2.43),(-6.5,5.05,2.43),.105,.105,4)
wood.box((-6.5,4.2,2.42),(.13,1.65,.12))
for x in [-7.9,-5.1]:wood.beam((x,1.8,2.43),(x+(.5 if x<-6 else -.5),2.6,2.43),.075,.075,4)
# Recessed door and diamond-paned windows on south and east facade.
wood.box((-7.4,1.04,2.45),(.86,1.62,.1))
for i in range(5):wood.box((-7.76+i*.18,1.04,2.51),(.025,1.52,.025),tile=12)
for y in [.58,1.45]:wood.box((-7.4,y,2.53),(.86,.07,.045))
for cx,cy in [(-5.5,2.55),(-7.45,2.65)]:
 glass.box((cx,cy,2.48),(.79,.82,.055))
 for x in [cx-.46,cx+.46]:wood.box((x,cy,2.52),(.10,1.02,.10))
 for y in [cy-.47,cy+.47]:wood.box((cx,y,2.52),(1.02,.10,.1))
 for k in range(-2,3):
  off=k*.22
  for sign in [-1,1]:
   # clipped leaded diamonds
   lo=max(-.37,(-.37-off)/sign) if sign==1 else max(-.37,(.37-off)/sign)
   hi=min(.37,(.37-off)/sign) if sign==1 else min(.37,(-.37-off)/sign)
   if hi>lo:wood.beam((cx+lo,cy+sign*lo+off,2.53),(cx+hi,cy+sign*hi+off,2.53),.012,.012,4)
# Timber brackets, trim and individually modeled slope tiles.
for side in [-1,1]:
 for course in range(10):
  t0=course/10;t1=(course+1.13)/10;x0=-6.5+side*(.03+2.02*t0);x1=-6.5+side*(.03+2.02*t1);y0=5.17-1.69*t0;y1=5.17-1.69*t1
  for col in range(12):
   z=-1.65+col*.36+(course%2)*.17
   if z>2.6:continue
   jitter=R.uniform(-.014,.014)
   face=[(x0,y0+jitter,z),(x1,y1+jitter,z),(x1,y1+jitter,z+.35),(x0,y0+jitter,z+.35)]
   roof.face(face if side<0 else list(reversed(face)))
   # lower lip gives real tiny shadow at each course
   roof.face([(x1,y1+jitter,z),(x1,y1+jitter-.045,z),(x1,y1+jitter-.045,z+.35),(x1,y1+jitter,z+.35)])
 for z in [-1.65,2.7]:wood.beam((-6.5,5.18,z),(-6.5+side*2.11,3.43,z),.09,.09,6)
wood.beam((-6.5,5.2,-1.7),(-6.5,5.2,2.78),.14,.14,8)
# Chimney stone courses and copper cap.
for row in range(7):
 for col in range(2):base.stone((-7.4+(col-.5)*.30,4.4+row*.22,-.3),(.30,.22,.49))
base.box((-7.4,5.93,-.3),(.77,.12,.69))
# Breakfast sideboard sits behind table, inside inn footprint. Open air above reserved table.
furn=Mesh('Breakfast_Table','Wood');cer=Mesh('Breakfast_Crockery','Ceramic');linen=Mesh('Breakfast_Linen','Linen');cop=Mesh('Inn_Copper','Copper')
furn.box((-4,.715,3),(1.48,.13,.76))
for x in [-4.57,-3.43]:
 for z in [2.76,3.24]:furn.box((x,.35,z),(.1,.70,.1))
for z in [2.72,3.28]:furn.box((-4,.55,z),(1.32,.09,.06))
# Runner leaves player bowl point clear, woven geometric texture.
linen.box((-4.42,.786,3),(.30,.014,.72));linen.box((-4.42,.62,3.4),(.30,.32,.012))
# Shelves and bowls contained at facade, not keeper position (-5,3).
furn.box((-5.62,.96,2.37),(.68,1.02,.07))
for y in [.58,.96,1.30]:
 furn.box((-5.62,y,2.43),(.68,.055,.15))
 for j in range(3):
  x=-5.85+j*.23
  cer.beam((x,y+.03,2.43),(x,y+.12,2.43),.083,.07,12)
# Lathed vessel profile, including hollow mouth and handles as curves made of beams.
def lathe(mesh,c,profile,n=16):
 cx,cy,cz=c
 for (h,r),(h2,r2) in zip(profile,profile[1:]):
  for i in range(n):
   a=i*math.tau/n;b=(i+1)*math.tau/n
   mesh.face([(cx+r2*math.cos(a),cy+h2,cz+r2*math.sin(a)),(cx+r2*math.cos(b),cy+h2,cz+r2*math.sin(b)),(cx+r*math.cos(b),cy+h,cz+r*math.sin(b)),(cx+r*math.cos(a),cy+h,cz+r*math.sin(a))])
lathe(cop,(-4.0,.79,2.79),[(0,.13),(.04,.16),(.26,.14),(.31,.10),(.36,.105),(.37,.08),(.32,.08)])
# Signature hanging copper kettle on east corner, easily visible from south.
wood.box((-4.23,3.08,2.25),(1.1,.12,.12));wood.beam((-4.66,2.56,2.25),(-3.85,3.04,2.25),.047,.047,6)
for x in [-4.15,-3.78]:cop.beam((x,3.03,2.25),(x,2.72,2.25),.018,.018,6)
lathe(cop,(-3.98,2.16,2.25),[(0,.15),(.06,.24),(.32,.25),(.40,.17),(.44,.18),(.47,.08),(.50,.035)],20)
for i in range(12):
 a=i*math.pi/12;b=(i+1)*math.pi/12
 cop.beam((-3.98+.23*math.cos(a),2.61+.20*math.sin(a),2.25),(-3.98+.23*math.cos(b),2.61+.20*math.sin(b),2.25),.025,.025,6)
cop.beam((-3.76,2.35,2.25),(-3.59,2.53,2.25),.06,.045,10)
# East flank window and flower sill; complete building from walking viewpoints.
glass.box((-4.51,2.49,.43),(.05,.88,.88))
for z in [-.07,.93]:wood.box((-4.46,2.49,z),(.11,1.07,.095))
for y in [1.99,2.99]:wood.box((-4.46,y,.43),(.11,.095,1.08))
for z in [.15,.43,.71]:wood.box((-4.43,2.49,z),(.025,.88,.025))
wood.box((-4.39,2.49,.43),(.03,.035,.88))
wood.box((-4.35,1.92,.43),(.31,.18,1.12))
wood.box((-4.16,2.03,.43),(.045,.04,1.18))
for z in [-.15,1.01]:wood.box((-4.31,2.03,z),(.34,.04,.045))
# A small linen weather hood belongs to the doorway, away from the keeper's
# orthographic sightline. Nothing projects over (-5,3).
for j in range(8):
 x=-7.95+j*.145
 linen.face([(x,2.40,2.39),(x+.145,2.40,2.39),(x+.145,2.22,2.71),(x,2.22,2.71)])
 linen.face([(x,2.22,2.71),(x+.145,2.22,2.71),(x+.145,2.14,2.71),(x,2.14,2.71)])
wood.beam((-7.95,2.36,2.4),(-7.95,2.20,2.71),.033,.033,6)
wood.beam((-6.79,2.36,2.4),(-6.79,2.20,2.71),.033,.033,6)
# A folded drying cloth hangs flat against the western wall, not in the lane.
wood.beam((-8.43,1.3,.3),(-8.43,1.3,1.35),.025,.025,6)
linen.face([(-8.45,1.28,.55),(-8.45,1.28,1.15),(-8.45,.83,1.17),(-8.45,.91,.52)])
# Two coopered casks west of the navigable rectangle, beside the inn.
for x,z,r,h in [(-8.85,2.0,.25,.62),(-8.85,2.58,.21,.5)]:
 lathe(furn,(x,.03,z),[(0,r*.84),(.08,r),(.45*h,r*1.08),(h-.08,r),(h,r*.86)],14)
 for y in [.11,h*.52,h-.05]:lathe(cop,(x,.03,z),[(y,r*1.035),(y+.025,r*1.035)],14)
 furn.beam((x,h+.025,z),(x,h+.045,z),r*.86,r*.86,14)
for mesh in [pl,wood,roof,glass,base,furn,cer,linen,cop]:mesh.finish()
# Trees: twisted forked trunks, hundreds of open sprigs in layered crowns, real branch silhouette.
def leaf(mesh,p,size,angle,tilt=0,tile=None):
 p=Vector(p);u=Vector((math.cos(angle),tilt,math.sin(angle))).normalized()*size;v=Vector((-math.sin(angle)*.7,.72,math.cos(angle)*.7)).normalized()*size
 mesh.face([p-u-v,p+u-v,p+u+v,p-u+v],tile=tile)
def tree(x,z,scale,tag,fruit=True,far=False):
 o=(x,0,z);tr=Mesh('trunk_'+tag,'Wood',12,origin=o);le=Mesh('canopy_'+tag,'Leaves',origin=o)
 yaw=R.uniform(0,math.tau) if far else (0 if tag=='main' else -.47)
 species=R.randrange(3) if far else 0
 spread=[1.,1.27,.79][species];stretch=[1.,.89,1.18][species]
 bend=R.uniform(-.44,.44) if far else .12
 def p(dx,y,dz):
  dx*=spread;dz*=spread;y*=stretch
  return (x+(dx*math.cos(yaw)+dz*math.sin(yaw))*scale,y*scale,z+(-dx*math.sin(yaw)+dz*math.cos(yaw))*scale)
 chain=[(0,0,0),(.06+bend*.2,.55,.02),(.03+bend*.55,1.15,-.04),(.19+bend,1.9,.02),(.48+bend*1.1,2.6,.12),(.78+bend,3.3,.22)]
 rings=[]
 for i,(dx,y,dz) in enumerate(chain):
  radius=scale*(.30-i*.038)*R.uniform(.93,1.06)
  rings.append([Vector(p(dx,y,dz))+Vector((math.cos(a*math.tau/12)*radius,0,math.sin(a*math.tau/12)*radius)) for a in range(12)])
 for j in range(len(rings)-1):
  for i in range(12):tr.face([rings[j][i],rings[j+1][i],rings[j+1][(i+1)%12],rings[j][(i+1)%12]])
 for j in range(7):
  a=j*math.tau/7+R.uniform(-.15,.15);tr.beam(p(0,.25,0),p(math.cos(a)*R.uniform(.49,.72),.025,math.sin(a)*.58),.16*scale,.035*scale,7)
 # Distinct overlapping domes, with low shaded boughs and separate sunlit ridges.
 clusters=[(-1.25,2.92,.18,.70,2),(-.75,3.62,-.7,.68,5),(.20,4.03,-.44,.72,8),(1.05,3.64,-.25,.73,7),(1.55,3.05,.45,.62,4),(.45,2.86,.92,.65,2),(-.73,2.60,.84,.52,1),(-1.25,3.83,.60,.57,7),(.40,4.28,.50,.58,9),(-.15,3.45,.12,.63,4)]
 for k,(dx,y,dz,radius,shade) in enumerate(clusters):
  y+=R.uniform(-.15,.15) if far else 0
  tr.beam(p(.1,1.7+R.uniform(-.15,.15),0),p(dx,y-.15,dz),.085*scale,.022*scale,7)
  for j in range(3):
   a=j*math.tau/3+k;tr.beam(p(dx,y-.15,dz),p(dx+math.cos(a)*radius*.7,y+.12,dz+math.sin(a)*radius*.7),.023*scale,.007*scale,5)
  for j in range(37 if far else 103):
   a=R.uniform(0,math.tau);rr=math.sqrt(R.random())*radius;dome=.48*math.sqrt(max(0,1-(rr/radius)**2));dy=dome+R.uniform(-.17,.15)
   colour=max(0,min(9,shade+(1 if dy>.28 else -1)+R.choice([-1,0,0,1])))
   if far and species==2:colour=[0,10,10,11,11,12,12,13,13,13][colour]
   leaf(le,p(dx+math.cos(a)*rr,y+dy,dz+math.sin(a)*rr*.86),scale*R.uniform(.17,.28),R.uniform(0,math.tau),R.uniform(-.6,.6),tile=colour)
 tr.finish();le.finish()
 if fruit:
  # South-facing hanging fruit deliberately below leaf envelope for legibility.
  for k,(dx,y,dz) in enumerate([(-1.0,2.48,.45),(-.82,2.42,.52),(.45,2.57,.9),(.63,2.51,.94),(1.46,2.98,.49),(1.65,2.90,.47),(-.41,3.12,-.65),(.55,3.35,.60),(1.10,3.42,.71)]):
   fr=Mesh('ripe_plum_'+tag+'_%02d'%k,'Plum',tile=6 if tag=='secondary' else 7);c=Vector(p(dx,y-.23,dz+.12));radius=(.094 if tag=='secondary' else .115)*scale
   # Low latitude-ring fruit with rounded crown, not cubic tokens.
   lathe(fr,c,[(-radius*.95,.025*scale),(-radius*.6,radius*.83),(0,radius),(.065*scale,radius*.8),(.10*scale,.025*scale)],12)
   fr.tile=12;fr.beam(c+Vector((0,.08*scale,0)),c+Vector((.025*scale,.18*scale,0)),.012*scale,.008*scale,5);fr.finish()
tree(5,-4,1,'main');tree(8,-2,.85,'secondary')
# Meadow apron, flowering grasses and distant woods. Nothing solid blocks unnamed playable cells.
plants=Mesh('Meadow_Herbs','Leaves');flowers=Mesh('Meadow_Flowers','Ceramic',14);grass=Mesh('Meadow_GrassBlades','Terrain')
# Discontinuous stepping stones and tiny verge clumps articulate the branch path
# without forming a curb, a collider or a regular stamped border.
for j in range(15):
 x=.45+j*.60;z=1.1+.42*math.sin(x*.65)+.035*x
 if j%3!=2:
  stones.stone((x,.012,z+R.uniform(-.14,.14)),(R.uniform(.14,.27),.028,R.uniform(.13,.24)),tile=2)
 if j%2==0:
  for side in [-1,1]:
   cx=x+R.uniform(-.12,.12);cz=z+side*R.uniform(.48,.66)
   for k in range(4):leaf(plants,(cx+R.uniform(-.1,.1),.07,cz+R.uniform(-.08,.08)),R.uniform(.06,.10),R.uniform(0,math.tau),tile=3+k%2)
for x,z in [(-7.9,3.02),(-7.3,3.55),(-6.5,3.85),(-5.5,3.68),(-4.8,3.77),(-3.1,3.62),(-2.4,3.25)]:
 for k in range(2):
  stones.stone((x+k*.17,.012,z+R.uniform(-.12,.12)),(.17,.028,.15),tile=2)
for k in range(820):
 x=R.uniform(-15,15);z=R.uniform(-19,12)
 if abs(x-(-.7+math.sin(z*.13)*.6))<1.9 or (-8.8<x<-3.1 and -1.8<z<5):continue
 if -8<x<8 and -5<z<5 and R.random()<.77:continue
 h=R.uniform(.07,.22)
 for j in range(3):
  a=R.random()*math.tau;grass.face([(x-.025,0,z),(x+.025,0,z),(x+math.cos(a)*.08,h,z+math.sin(a)*.08)],tile=0)
 if k%4==0:
  leaf(plants,(x,.12,z),.13,R.random()*math.tau)
  for j in range(3):
   a=j*2.1;cx=x+math.cos(a)*.07;cz=z+math.sin(a)*.07
   flowers.face([(cx-.035,h,cz-.035),(cx+.035,h,cz-.035),(cx+.035,h,cz+.035),(cx-.035,h,cz+.035)],tile=9 if k%3 else 14)
 if k%29==0:
  for j in range(5):leaf(plants,(x,.08+j*.075,z),.07,R.random()*math.tau)
# A planted sill adds a small lived-in detail without occupying walkable floor.
for j in range(9):
 z=.0+j*.105;leaf(plants,(-4.32,2.08,z),.14,j*.9)
 if j%2==0:
  flowers.face([(-4.25,2.17,z-.045),(-4.17,2.17,z-.045),(-4.17,2.17,z+.045),(-4.25,2.17,z+.045)],tile=9)
plants.finish();flowers.finish();grass.finish();stones.finish()
# Rock outcrops only outside navigation rectangle.
rocks=Mesh('Meadow_Outcrops','Stone')
for x,z in [(-10,-4),(11,-7),(-10,6),(11,4),(15,-12),(-12,-13),(6,-11)]:
 for k in range(4):rocks.stone((x+R.uniform(-.4,.4),.10,z+R.uniform(-.3,.3)),(R.uniform(.4,1),R.uniform(.25,.65),R.uniform(.4,.8)))
rocks.finish()
# Far trees join into two inexpensive combined meshes after construction.
for k,(x,z,s) in enumerate([(-11.5,-5,1.35),(-12.0,1.5,1.45),(-13.5,7,1.4),(12.4,-6,1.5),(14,0,1.4),(13,6,1.25)]):
 tree(x,z,s,'far_side_%02d'%k,False,True)
for k in range(33):
 x=R.uniform(-29,29);z=R.uniform(-34,-15)
 if abs(x+.7)<2.5:continue
 tree(x,z,R.uniform(1.25,1.9),'far_%02d'%k,False,True)
# Continuous low-frequency rolling terrain rather than repeated triangular peaks.
hills=Mesh('Horizon_Hills','Terrain',13)
for layer in range(3):
 front=-42-layer*20
 def elevation(x,depth):
  return (3.4+layer*2.1+2.1*math.sin(x*.055+layer*1.8)+1.15*math.sin(x*.105+layer*.7))*math.sin(math.pi*depth/6)
 for iz in range(6):
  z0=front-iz*7;z1=z0-7
  for ix in range(90):
   x0=-90+ix*2;x1=x0+2
   hills.face([(x0,elevation(x0,iz),z0),(x1,elevation(x1,iz),z0),(x1,elevation(x1,iz+1),z1),(x0,elevation(x0,iz+1),z1)],uv=[(.48,.48)]*4,tile=13 if layer==0 else 15)
hills.finish()
# Join distant meshes by material to hold draw overhead down.
for prefix,name in [('trunk_far_','Horizon_Trunks'),('canopy_far_','Horizon_Woods')]:
 obs=[o for o in bpy.context.scene.objects if o.type=='MESH' and o.name.startswith(prefix)]
 bpy.ops.object.select_all(action='DESELECT')
 for o in obs:o.select_set(True)
 bpy.context.view_layer.objects.active=obs[0];bpy.ops.object.join();obs[0].name=name
# Resolve convex caps before tangent export; Blender's tangent calculator accepts
# triangles/quads only. UV loop data stays attached through bmesh triangulation.
import bmesh
for ob in bpy.context.scene.objects:
 if ob.type!='MESH':continue
 bm=bmesh.new();bm.from_mesh(ob.data)
 ngons=[face for face in bm.faces if len(face.verts)>4]
 if ngons:bmesh.ops.triangulate(bm,faces=ngons)
 bm.to_mesh(ob.data);bm.free();ob.data.update()
# Scene export has meshes only. Lighting/cameras retained in blend authoring source, not runtime GLB.
bpy.ops.object.select_all(action='DESELECT')
for o in bpy.context.scene.objects:
 if o.type=='MESH':o.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'orchard.glb'),export_format='GLB',use_selection=True,export_yup=True,export_normals=True,export_tangents=True,export_texcoords=True,export_materials='EXPORT',export_cameras=False,export_lights=False,export_extras=True)
# Blender 4.5 dithered transparency maps to BLEND; leaf silhouettes use deterministic
# alpha clipping in the runtime instead, preserving solid depth and shadow writes.
import struct
fp=OUT/'orchard.glb';data=fp.read_bytes();jslen=struct.unpack_from('<I',data,12)[0];doc=json.loads(data[20:20+jslen]);tail=data[20+jslen:]
for material in doc['materials']:
 if material['name']=='Lane_Leaves':material['alphaMode']='MASK';material['alphaCutoff']=.5;material['doubleSided']=True
payload=json.dumps(doc,separators=(',',':')).encode();payload+=b' '*((-len(payload))%4)
fp.write_bytes(struct.pack('<4sII',b'glTF',2,20+len(payload)+len(tail))+struct.pack('<I4s',len(payload),b'JSON')+payload+tail)
# Authoring preview: studio daylight with real shadows, useful high and low viewpoints.
world=bpy.context.scene.world;world.use_nodes=True;bg=next(n for n in world.node_tree.nodes if n.type=='BACKGROUND');bg.inputs[0].default_value=(.40,.54,.63,1);bg.inputs[1].default_value=.65
bpy.ops.object.light_add(type='SUN',location=(0,0,12));sun=bpy.context.object;sun.rotation_euler=(math.radians(27),math.radians(-24),math.radians(-28));sun.data.energy=2.2;sun.data.angle=.10
bpy.ops.object.camera_add();cam=bpy.context.object;bpy.context.scene.camera=cam
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True;scene.render.resolution_x=1400;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast'
def camera(pos,target,lens,path):
 cam.location=cv(pos);direction=Vector(cv(target))-cam.location;cam.rotation_euler=direction.to_track_quat('-Z','Y').to_euler();cam.data.type='PERSP';cam.data.lens=lens;scene.render.filepath=str(ART/path);bpy.ops.render.render(write_still=True)
camera((14,20,24),(-.5,1,-1),45,'overview.png')
camera((1.3,1.65,6.3),(2.5,1.8,-5),26,'eye_level.png')
camera((.2,22,17),(0,0,-.7),46,'overhead.png')
bpy.ops.wm.save_as_mainfile(filepath=str(ART/'orchard.blend'))
report={'generator':'blender-4.5 + bpy; tools/make_lane.py','source':'Original geometry and deterministic pixel atlas by OpenAI for Wanderstead; no third-party artwork','coordinates':'glTF Y-up; north -Z; scene origin 0,0,0; metres','fruit_prefix':'ripe_plum_main_','secondary_fruit_prefix':'ripe_plum_secondary_','shake_pivot':'canopy_main at (5,0,-4)','materials':[m.name for m in M.values()]+[earth.name],'mesh_nodes':len([o for o in scene.objects if o.type=='MESH']),'triangles':sum(sum(len(p.vertices)-2 for p in o.data.polygons) for o in scene.objects if o.type=='MESH'),'atlas':'lane_atlas.png 512x512 with 16 original 128px tiles; closest sampling; Lane_Leaves alpha silhouette','terrain':'lane_terrain.png 2048x1792 world-mapped pixel field with soft shoulders and wheel wear','roughness':'lane_roughness.png linear PBR roughness'}
report['normal_map']='lane_normal.png, original tangent-space grain/pits, non-colour, glTF normalTexture'
report['secondary_fruit_maturity']='unripe green; shared Lane_Plum material with green atlas UVs'
report['foliage']='lane_foliage.png: sixteen authored sun/shade/species sprig groups, shared MASK material'
report['revision']='Bevy 11/12 screenshot revision: organic worn paths, retracted keeper shelf, grouped crowns, varied woodland and rolling 3D hills'
(ART/'source.json').write_text(json.dumps(report,ensure_ascii=False,indent=2));print('LANE_DONE',json.dumps(report))
