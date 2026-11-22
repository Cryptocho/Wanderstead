"""Original limited-palette town surfaces; imported by Blender's make_lane.py.
All pixels are deterministic source artwork authored here, not image filters on a
render. Arrays are linear RGBA, bottom row first, ready for bpy image storage.
"""
import math
import numpy as np

PALETTES = {
 'Paving':['9f9d8f','c7bfa9','d8ceb4','e6dcc0','bcb8aa'],
 'BackstreetSetts':['829ca4','a7bcc0','bcccc6','d8ddc7','9db3b7'],
 'Slate':['45697d','628da2','81aabd','accbd1','567b91'],
 'WeatheredSlate':['427f83','62a0a0','88baba','b0d4c9','518e96'],
 'CopperRoof':['3d8984','61aaa0','8dc8b0','bbdcc0','4e9691'],
 'Plaster':['c7bd93','ded5b0','eee2bc','fff0d0','d9cda3'],
 'WorkshopLime':['b5c9b2','d0ddc0','e4e9ca','f4f1d6','c3d2b5'],
 'Wood':['885135','ac7248','c08c58','dfae73','9c613e'],
 'Masonry':['a1a493','c0c3ab','d8d8ba','eee6c8','b2b8a2'],
 'CanalMasonry':['9caeaa','b8c7b9','d6ddc4','edeaca','aabdb2'],
 'Stone':['b5b49c','d0d0b2','e8e2c2','fff0d0','c4c4a9'],
 'Crimson':['9b473e','bd5446','d96e51','ec9a69','ae5141'],
 'Banner':['9c443b','ba5748','d87553','edb16d','aa4d40'],
 'Linen':['bfb895','d9d2ad','eee7c5','fff5d7','cbc39e'],
 'Copper':['785545','a76b44','c28b54','ebbc75','955e43'],
 'Earth':['81916a','9bac78','acbb83','c2cd95','8f9e6e'],
 'Ivy':['367c58','52955c','7db75f','b7d87b','438b57'],
 'Leaves':['367c58','52955c','7db75f','b7d87b','438b57'],
 'Petals':['b56477','d38693','edb2b5','ffe1d2','c87887'],
 'Blossom':['b56477','d38693','edb2b5','ffe1d2','c87887'],
 'Pottery':['a45d48','bd7756','d5966c','e9ba83','b16b4f'],
 'Ceramic':['428b94','62abb1','8dc8ca','c3e2d9','509ca4'],
 'Glass':['4a8c9a','79b6bd','b5d7d1','e7edcd','62a4af'],
}

def linear(hex_colour):
 rgb=np.array([int(hex_colour[i:i+2],16)/255 for i in (0,2,4)],dtype=np.float32)
 return np.where(rgb<=.04045,rgb/12.92,((rgb+.055)/1.055)**2.4)

def palette(name):
 return np.array([linear(c) for c in PALETTES[name]],dtype=np.float32)

def surface(name):
 """64px original hand-designed material motifs, or sixteen 64px street flags."""
 n=256 if name=='Paving' else 64
 yy,xx=np.mgrid[0:n,0:n];x=xx%64;y=yy%64
 colours=palette(name);idx=np.full((n,n),2,dtype=np.int32)
 if name in ('Paving','BackstreetSetts','Stone','Masonry','CanalMasonry'):
  if name=='Paving':
   # Multiple narrow stones inside each metre; quiet dotted corner registration
   # replaces the repeated kitchen-slab outline. Edges still align to gameplay.
   variant=xx//64*13+yy//64*29
   row=y//16;shift=(row%2)*13+(xx//64*7+yy//64*11)%9
   coord=x+shift; brick=coord//31
   u=coord%31;v=y%16;stone=(variant*17+row*31+brick*13+row*brick*3)%19
   idx[:]=2;idx[stone<5]=1;idx[stone==7]=4
   edge=(u<1)|(v<1)|((u<2)&(v<2))|((u>28)&(v>13)&(stone%3==1))
   idx[edge]=0;idx[(u==1)&~edge]=1;idx[(v==1)&~edge]=3
   idx[((u==29)&(v>5)&(v<11))|((v==15)&(u>4)&(u<26))]=1
   corner=((x<2)|(x>61))&((y<2)|(y>61));idx[corner]=3
  else:
   block=16 if name=='BackstreetSetts' else 32
   u=(x+(y//block%2)*block//2)%block;v=y%block
   idx[:]=2;idx[(u<1)|(v<1)]=0;idx[(u==1)|(v==1)]=3
   idx[(u==block-1)|(v==block-1)]=1
   idx[((x//4+y//7)%13==0)&(u>4)&(v>4)]=4
 elif name in ('Slate','WeatheredSlate','CopperRoof'):
  u=(x+(y//16%2)*8)%16;v=y%16
  idx[:]=2;idx[(u<1)|(v<2)]=0;idx[(u==1)|(v==2)]=3;idx[v>13]=1
  idx[(u>10)&(v>5)&(v<12)]=4
 elif name=='Wood':
  u=x%16;idx[:]=2;idx[u<1]=0;idx[u==1]=3;idx[u>13]=1
  idx[((x//3+y//12)%7==0)&(u>3)&(u<12)]=4
  idx[(abs(x-24)<3)&(abs(y-39)<4)]=0
 elif name in ('Crimson','Linen','Banner'):
  idx[:]=2;idx[(x//4)%8==0]=1;idx[(x%16==2)&(y%4<2)]=3
 elif name in ('Ivy','Leaves','Petals','Blossom'):
  return crown(name in ('Petals','Blossom'))
 elif name=='Earth':
  idx[:]=2;idx[((x//5+y//7)%9)==0]=1;idx[(x%17<3)&(y%13<2)]=3
 elif name in ('Pottery','Ceramic','Copper','Glass'):
  idx[:]=2;idx[x<10]=1;idx[(x>=10)&(x<16)]=3;idx[x>52]=4
 else:
  idx[:]=2;idx[((x//8+y//8*3)%17)==0]=1;idx[((x//6-y//9)%23)==0]=3
 data=np.ones((n,n,4),dtype=np.float32);data[:,:,:3]=colours[idx]
 return data

def worked_material(name,base,study):
 """Use a measured value relief, not a photograph pasted on the model. Re-ink the
 overlapping slate edges, timber rings and lime hairlines into restrained ramps."""
 lum=np.sqrt(study[:,:,:3]@np.array((.2126,.7152,.0722),dtype=np.float32))
 lo,hi=np.percentile(lum,[5,96])
 t=np.round(np.clip((lum-lo)/max(.015,hi-lo),0,1)*10)/10
 if name in ('Slate','WeatheredSlate','CopperRoof'):
  dark,light={'Slate':('627586','8da3ad'),'WeatheredSlate':('597d80','93aea3'),'CopperRoof':('567f78','a9bb97')}[name]
  # A roof already has steps, thickness and shadows. Its texture describes
  # scales and worn edges without putting a second hard black shadow on each tile.
  base[:,:,:3]=linear(dark)+(linear(light)-linear(dark))*t[:,:,None]
 elif name=='Paving':
  base[:,:,:3]*=(.78+.28*np.tile(t,(4,4)))[:,:,None]
 elif name in ('Wood','Plaster','WorkshopLime','Masonry','CanalMasonry','Stone'):
  base[:,:,:3]*=(.78+.30*t)[:,:,None]
 return base

def crown(blossom=False):
 """64x64 leaf-lobe cutout: broad contiguous clusters, crisp stepped outline."""
 n=64;yy,xx=np.mgrid[0:n,0:n];data=np.zeros((n,n,4),dtype=np.float32)
 pal=palette('Blossom' if blossom else 'Leaves')
 centres=[(18,22,15),(39,20,16),(48,33,12),(32,38,20),(15,39,12),(32,50,12)]
 mask=np.zeros((n,n),dtype=bool);height=np.zeros((n,n),dtype=np.float32)
 for cx,cy,r in centres:
  # 2px macro staircase keeps the silhouette deliberately pixel-authored.
  d=((xx//2*2-cx)/r)**2+((yy//2*2-cy)/r)**2
  mask|=d<1;height=np.maximum(height,np.clip(1-d,0,1))
 idx=np.ones((n,n),dtype=np.int32)
 idx[height>.18]=2
 idx[(height<.10)&(yy<30)]=0
 # Deliberate 5-9px leaf-pair brush clusters: contiguous paint masses with
 # a few readable curled leaf tips, rather than a flat lime disk or noise.
 for k,(cx,cy) in enumerate([(13,21),(29,17),(42,22),(18,30),(34,29),
                            (46,35),(11,40),(24,40),(38,42),(27,51),(37,53)]):
  broad=(xx>=cx-3)&(xx<cx+5)&(yy>=cy)&(yy<cy+3)
  tip=(xx>=cx+2)&(xx<cx+7)&(yy>=cy+2)&(yy<cy+5)
  idx[broad|tip]=3 if k%3 else 2
  shadow=(xx>=cx-2)&(xx<cx+4)&(yy>=cy-2)&(yy<cy)
  idx[shadow]=1 if k%3 else 0
 data[:,:,:3]=pal[idx];data[:,:,3]=mask.astype(np.float32)
 return data

def flower():
 n=32;yy,xx=np.mgrid[0:n,0:n];data=np.zeros((n,n,4),dtype=np.float32)
 def rect(x,y,w,h,c):
  data[y:y+h,x:x+w,:3]=linear(c);data[y:y+h,x:x+w,3]=1
 for x,y in [(6,12),(15,18),(24,11)]:
  rect(x,1,2,y,'4b8d60');rect(max(0,x-5),5,5,3,'83ba6c')
  rect(x+2,8,min(4,30-x),3,'b3d67d')
  rect(x-4,y,10,4,'dd7359');rect(x-2,y-2,6,8,'ec9870')
  rect(x,y+1,2,3,'fff0ac');rect(x-3,y+3,3,2,'ffcf91')
 return data

def maps(data,rough):
 """Subtle deliberately chunky relief; never derives random photogrammetry."""
 lum=data[:,:,:3].mean(axis=2);dy,dx=np.gradient(lum)
 normal=np.ones_like(data);vec=np.stack((-dx*.75,-dy*.75,np.ones_like(dx)),axis=-1)
 vec/=np.linalg.norm(vec,axis=2,keepdims=True);normal[:,:,:3]=vec*.5+.5
 r=np.ones_like(data);r[:,:,:3]=np.clip(rough+(lum-lum.mean())*.10,.1,1)[:,:,None]
 return normal,r
