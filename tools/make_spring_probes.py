"""Original analytic lighting environments, baked to standard RGBA16F DDS cubemaps.
Run: python3 tools/make_spring_probes.py (NumPy). No runtime convolution or readbacks.
Faces follow DDS order; Bevy reverses Z when looking up an environment-map reflection.
"""
from pathlib import Path
import math,struct
import numpy as np

OUT=Path(__file__).resolve().parents[1]/'assets/inks'

def normalize(v):return v/np.maximum(1e-9,np.linalg.norm(v,axis=-1,keepdims=True))
def smooth(a,b,x):
    t=np.clip((x-a)/(b-a),0,1);return t*t*(3-2*t)
def mix(a,b,t):return np.array(a)+(np.array(b)-a)*t[...,None]

def radiance(d,town):
    x,y,z=np.moveaxis(d,-1,0);az=np.arctan2(x,-z)
    if town:
        sky=mix((1.08,1.17,1.08),(.26,.60,1.13),smooth(.03,.94,y))
        slope=.11+.16*(1+np.sin(az*3.3+.75))*.5+.06*np.cos(az*9.)
        roofs=mix((.19,.25,.32),(.83,.34,.14),smooth(-.4,.7,np.cos(az*3.)))
        street=mix((.30,.22,.17),(.66,.57,.37),smooth(-.98,-.20,y))
        lower=mix(street,roofs,smooth(-.16,-.06,y));color=mix(lower,sky,smooth(slope-.025,slope+.015,y))
        lights=[((-.68,.58,.46),10,(1.22,.92,.55)),((.48,.12,.87),24,(.52,1.0,.93))]
    else:
        color=mix((.062,.115,.19),(.055,.090,.16),smooth(-.30,.85,y))
        color+=np.maximum(-y,0)[...,None]*np.array((.020,.24,.23))
        lights=[((.93,.15,-.34),30,(.17,2.8,3.0)),((-.72,.20,-.66),46,(3.9,1.25,.24)),((.06,.79,.60),19,(.99,1.48,2.16)),((-.85,.03,.53),50,(2.7,.97,.17))]
    for axis,hard,rgb in lights:
        axis=normalize(np.array(axis));color+=np.maximum(0,np.sum(d*axis,axis=-1))[...,None]**hard*np.array(rgb)
    return color

def directions(face,n):
    u,v=np.meshgrid(2*(np.arange(n)+.5)/n-1,2*(np.arange(n)+.5)/n-1);one=np.ones_like(u)
    dirs=[(one,-v,-u),(-one,-v,u),(u,one,v),(u,-one,-v),(u,-v,one),(-u,-v,-one)][face]
    return normalize(np.stack(dirs,axis=-1)*np.array((1,1,-1)))

def integrated(normal,rough,town):
    if rough==0:return radiance(normal,town)
    ref=np.zeros_like(normal);ref[...,1]=1;vertical=np.abs(normal[...,1])>.92;ref[vertical]=[1,0,0]
    right=normalize(np.cross(ref,normal));up=np.cross(normal,right)
    out=np.zeros_like(normal);weight=np.zeros_like(normal[...,:1]);count=128
    for j in range(count):
        u=(j+.5)/count;phi=math.tau*(int(f'{j:07b}'[::-1],2)+.5)/count
        c=math.sqrt(1.-u) if rough is None else math.sqrt((1.-u)/(1.+(rough**4-1.)*u))
        s=math.sqrt(1.-c*c);half=normal*c+(right*math.cos(phi)+up*math.sin(phi))*s
        d=half if rough is None else 2*c*half-normal
        w=1. if rough is None else max(0,2*c*c-1)
        if w:out+=radiance(d,town)*w;weight+=w
    return out/np.maximum(weight,.00001)

def bake(name,town,diffuse):
    size=12 if diffuse else 64;mips=1 if diffuse else 7
    # Legacy D3DFMT_A16B16G16R16F (fourcc 113), six faces, face-major mip chains.
    flags=0x100f | (0x20000 if mips>1 else 0);caps=0x1008|(0x400000 if mips>1 else 0)
    header=[124,flags,size,size,size*8,0,mips]+[0]*11+[32,4,113,0,0,0,0,0,caps,0xfe00,0,0,0]
    with (OUT/f'{name}.dds').open('wb') as f:
        f.write(b'DDS '+struct.pack('<31I',*header))
        for face in range(6):
            for level in range(mips):
                n=size>>level;rgb=integrated(directions(face,n),None if diffuse else level/(mips-1),town)
                rgba=np.ones((n,n,4),dtype='<f2');rgba[:,:,:3]=rgb;f.write(rgba.tobytes())
for town,place in [(True,'square'),(False,'cistern')]:
    for diff,suffix in [(True,'skyfill'),(False,'reflections')]:bake(f'{place}-{suffix}',town,diff)
