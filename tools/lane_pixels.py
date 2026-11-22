"""Original limited-palette town surfaces; imported by Blender's make_lane.py.
All pixels are deterministic source artwork authored here, not image filters on a
render. Arrays are linear RGBA, bottom row first, ready for bpy image storage.
"""
import math
import numpy as np

PALETTES = {
 'Paving':['736963','867160','a08873','b79a82','938986'],
 'BackstreetSetts':['716d69','7c7c78','8e8d88','a29f98','84898d'],
 'Slate':['45697d','628da2','81aabd','accbd1','567b91'],
 'WeatheredSlate':['427f83','62a0a0','88baba','b0d4c9','518e96'],
 'CopperRoof':['3d8984','61aaa0','8dc8b0','bbdcc0','4e9691'],
 'Plaster':['918172','ab9786','c2ae96','d9c4a9','ab9b8d'],
 'WorkshopLime':['878477','aba897','c0bba5','d5cdb2','a9aa99'],
 'Wood':['885135','ac7248','c08c58','dfae73','9c613e'],
 'Masonry':['7d7169','998675','a9927e','c1aa8e','a09689'],
 'CanalMasonry':['81766e','998879','ac9987','c1ae98','a09a92'],
 'Stone':['918477','a99985','b9a58c','cdb79a','ad9f91'],
 'Crimson':['9b473e','bd5446','d96e51','ec9a69','ae5141'],
 'Banner':['9c443b','ba5748','d87553','edb16d','aa4d40'],
 'Linen':['bfb895','d9d2ad','eee7c5','fff5d7','cbc39e'],
 'Copper':['785545','a76b44','c28b54','ebbc75','955e43'],
 'Earth':['596643','718052','85945f','a4ac77','68774c'],
 'Ivy':['355d38','4e773f','698d49','a8ba75','577e42'],
 'Leaves':['355d38','4e773f','698d49','a8ba75','577e42'],
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

def street(name):
 """32m painting at 64px/m: close limestone hues, uniquely cut worn setts."""
 n=2048;data=np.ones((n,n,4),dtype=np.float32)
 pal=palette(name);rng=np.random.default_rng(191173)
 yy,xx=np.mgrid[0:n,0:n]
 # Broad mineral beds cross many stones. Neither hue nor patina repeats per cell.
 bed=(np.sin(xx/193.+np.sin(yy/347.))+.65*np.cos(yy/257.)+.38*np.sin((xx-yy)/109.))/2.03
 repair=(np.sin(xx/307.+1.7)*np.cos(yy/241.-.8))>.81
 data[:,:,:3]=pal[0]
 y=-17
 while y<n:
  height=int(rng.integers(8,16));x=-int(rng.integers(7,31))
  if rng.random()<.065:height=int(rng.integers(16,20))
  while x<n:
   width=int(rng.integers(15,32))
   if rng.random()<.055:width=int(rng.integers(33,43))
   x0=max(0,x+1);x1=min(n,x+width);y0=max(0,y+1);y1=min(n,y+height)
   if x1>x0 and y1>y0:
    sy,sx=np.mgrid[y0:y1,x0:x1];u=(sx-x)/width;v=(sy-y)/height
    rock=float(bed[min(n-1,max(0,y+height//2)),min(n-1,max(0,x+width//2))])
    tint=np.clip(.48+rock*.20+rng.normal(0,.19),.04,.96)
    col=pal[1]*(1-tint)+pal[3]*tint
    if repair[min(n-1,max(0,y+height//2)),min(n-1,max(0,x+width//2))] and rng.random()<.30:
     col=col*.35+pal[4]*.65
    worn=np.maximum(0,1-((u-.49)*1.65)**2-((v-.5)*1.7)**2)
    mineral=np.zeros_like(worn)
    for _ in range(3):
     cu,cv=rng.uniform(0,1,2);ru,rv=rng.uniform(.14,.44,2)
     mineral+=rng.uniform(-.10,.085)*np.maximum(0,1-((u-cu)/ru)**2-((v-cv)/rv)**2)
    mineral=np.round(mineral*180)/180
    block=col[None,None,:]*(.974+.035*worn[:,:,None]+mineral[:,:,None])
    # Chisel marks and uneven rounded corners, without a black perimeter.
    left,right,top,bottom=rng.integers(1,4,size=4)
    chip=((sx-x)+(sy-y)<left+1)|((x+width-sx)+(y+height-sy)<right+1)|((sx-x)+(y+height-sy)<top)|((x+width-sx)+(sy-y)<bottom)
    if rng.random()<.26:
     notch=(sx==x0)&(abs(sy-(y+height*rng.uniform(.2,.8)))<1.5)
     chip|=notch
    block[chip]=pal[0]
    bevel=((sy==y0)|(sx==x0))&~chip;block[bevel]*=1.065
    block[(sy==y1-1)&~chip]*=.92
    if rng.random()<.08:
     scar=(abs((sx-x)-width*.62-(sy-y)*.12)<.6)&(v>.25)&(v<.72)
     block[scar]*=.95
    data[y0:y1,x0:x1,:3]=block
   x+=width
  y+=height
 return data

def limestone(name):
 """Warm quarried wall courses and plaster with quiet mineral islands."""
 n=64;yy,xx=np.mgrid[0:n,0:n];pal=palette(name)
 rng=np.random.default_rng(513+sum(map(ord,name)))
 data=np.ones((n,n,4),dtype=np.float32);tone=np.zeros((n,n),dtype=np.float32)
 for _ in range(24):
  x,y=rng.uniform(-8,72,2);rx,ry=rng.uniform(4,15,2)
  tone+=rng.uniform(-.07,.06)*np.maximum(0,1-((xx-x)/rx)**2-((yy-y)/ry)**2)
 data[:,:,:3]=pal[2]*(.99+np.round(tone*120)[:,:,None]/120)
 if name not in ('Plaster','WorkshopLime'):
  row=yy//16;stagger=xx+(row%2)*15;u=stagger%32;v=yy%16
  values=np.array([[.845,1.00],[.93,1.08],[1.035,.90],[.985,.865]])
  data[:,:,:3]*=values[row,(stagger//32)%2,None]
  joints=(u==0)|(v==0)|((u<3)&(v<3)&(u+v<3))
  data[joints,:3]=pal[0]*1.09
  data[(v==1)&~joints,:3]*=1.07
  data[(v==15)&~joints,:3]*=.94
 return data

def surface(name):
 """Small original material motifs; paving uses a unique world-space painting."""
 if name in ('Paving','BackstreetSetts'):return street(name)
 if name in ('Stone','Masonry','CanalMasonry','Plaster','WorkshopLime'):return limestone(name)
 n=64
 yy,xx=np.mgrid[0:n,0:n];x=xx%64;y=yy%64
 colours=palette(name);idx=np.full((n,n),2,dtype=np.int32)
 if name in ('Slate','WeatheredSlate','CopperRoof'):
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
  # Broad brush groups retain roof-scale patina; isolated painting pixels do
  # not turn into repeated glitter under the live sun.
  t=t.reshape(32,2,32,2).mean(axis=(1,3)).repeat(2,axis=0).repeat(2,axis=1)
  dark,light={'Slate':('627586','8da3ad'),'WeatheredSlate':('597d80','93aea3'),'CopperRoof':('567f78','a9bb97')}[name]
  # A roof already has steps, thickness and shadows. Its texture describes
  # scales and worn edges without putting a second hard black shadow on each tile.
  base[:,:,:3]=linear(dark)+(linear(light)-linear(dark))*t[:,:,None]
 elif name=='Wood':
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

def maps(data,rough,relief=.75):
 """Chunky mineral relief, with brighter worn centres slightly more polished."""
 lum=data[:,:,:3].mean(axis=2);dy,dx=np.gradient(lum)
 normal=np.ones_like(data);vec=np.stack((-dx*relief,-dy*relief,np.ones_like(dx)),axis=-1)
 vec/=np.linalg.norm(vec,axis=2,keepdims=True);normal[:,:,:3]=vec*.5+.5
 r=np.ones_like(data);r[:,:,:3]=np.clip(rough-(lum-lum.mean())*.32,.1,1)[:,:,None]
 return normal,r
