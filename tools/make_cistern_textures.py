#!/usr/bin/env python3
"""Author original low-density sanctuary PBR tiles; no photographic input.

Run with Python + Pillow + NumPy before make_cistern.py. Each deliberately drawn
pixel is retained in the exported maps; heights describe carving, not luminance.
"""
from pathlib import Path
from PIL import Image, ImageDraw
import numpy as np

ROOT=Path(__file__).resolve().parents[1]
ART=ROOT/'art/cistern'
OUT=ROOT/'assets/cistern'
OUT.mkdir(parents=True,exist_ok=True)
ART.mkdir(parents=True,exist_ok=True)

def maps(name,size,base,rough,metal=0):
    return (Image.new('RGB',(size,size),base),
            Image.new('L',(size,size),128),
            Image.new('L',(size,size),round(rough*255)),
            Image.new('L',(size,size),round(metal*255)))

def draw_tile(name):
    size=64 if name in ('paving','banner','mosaic','spring_saint') else 32
    colors={
        'paving':(150,179,192),'masonry':(157,188,206),
        'spring_saint':(215,225,208),'banner':(112,58,93),
        'mosaic':(217,225,208),'vent':(167,129,64),
        'walnut':(139,86,68),'acanthus':(211,219,203)}
    images=maps(name,size,colors[name],.72 if name not in ('vent','mosaic') else .42,.78 if name=='vent' else 0)
    c,h,r,m=[ImageDraw.Draw(im) for im in images]
    if name in ('paving','masonry','spring_saint'):
        # Broad, sparse stepped mineral strata, without random per-pixel noise.
        for pts,col,height in [
            ([(0,5),(9,5),(9,3),(19,3),(19,8),(13,8),(13,11),(0,11)],(168,194,204),132),
            ([(size-15,size-10),(size-5,size-10),(size-5,size-15),(size,size-15),(size,size-4),(size-15,size-4)],(132,166,184),125),
            ([(size//3,size//2),(size//3+8,size//2),(size//3+8,size//2+2),(size//3+3,size//2+2),(size//3+3,size//2+5),(size//3,size//2+5)],(182,201,204),130)]:
            if name=='spring_saint':col=tuple(min(246,v+35) for v in col)
            c.polygon(pts,fill=col);h.polygon(pts,fill=height)
        if name=='paving':
            # Two small chisel scratches, not a second tile grid.
            c.line([(42,13),(46,13),(46,16),(50,16)],fill=(112,148,167),width=1)
            h.line([(42,13),(46,13),(46,16),(50,16)],fill=116,width=1)
            r.rectangle((0,45,63,59),fill=145)
    elif name=='banner':
        c.rectangle((3,0,6,63),fill=(228,192,109));c.rectangle((57,0,60,63),fill=(228,192,109))
        # The spring-keeper's jug and three stepped streams: original heraldry.
        c.polygon([(25,13),(38,13),(41,20),(39,32),(35,36),(27,36),(23,30),(22,20)],fill=(238,218,157))
        c.line([(39,19),(47,19),(47,29),(40,29)],fill=(238,218,157),width=3)
        c.rectangle((28,10,35,13),fill=(238,218,157))
        for x in (22,31,40):
            c.line([(x,40),(x,45),(x-3,45),(x-3,51),(x+1,51),(x+1,57)],fill=(109,193,179),width=2)
        for x in range(0,64,8):
            h.rectangle((x,0,x+2,63),fill=133)
            r.rectangle((x,0,x+2,63),fill=212)
    elif name=='mosaic':
        for y in range(0,64,8):
            for x in range(0,64,8):
                col=[(215,224,205),(115,175,173),(64,135,153),(224,200,131)][((x//8)*3+y//8)%4]
                c.rectangle((x+1,y+1,x+6,y+6),fill=col)
                h.rectangle((x+1,y+1,x+6,y+6),fill=148)
        r.rectangle((0,0,63,63),fill=146)
    elif name=='vent':
        c.rectangle((2,2,29,29),outline=(232,191,104),width=2)
        for y in (8,14,20):
            c.rectangle((7,y,24,y+2),fill=(50,83,83))
            h.rectangle((7,y,24,y+2),fill=65)
            m.rectangle((7,y,24,y+2),fill=40)
        for p in [(4,4,8,6),(23,24,28,28),(3,18,5,24)]:
            c.rectangle(p,fill=(70,159,146));r.rectangle(p,fill=191);m.rectangle(p,fill=65)
    elif name=='walnut':
        for x in (0,8,16,24):
            c.line((x,0,x,31),fill=(90,59,61),width=1)
            h.line((x,0,x,31),fill=105,width=1)
            c.line((x+2,2,x+2,29),fill=(185,118,79),width=1)
        c.line([(11,6),(13,6),(13,13),(11,13),(11,6)],fill=(111,66,57),width=1)
    else:
        # Single stepped water-leaf meander, readable on a chamfered block.
        for x in (0,16):
            pts=[(x+2,26),(x+2,8),(x+6,8),(x+6,4),(x+13,4),(x+13,12),(x+9,12),(x+9,20),(x+13,20),(x+13,26)]
            c.line(pts,fill=(135,173,177),width=2);h.line(pts,fill=100,width=2)
            c.line([(x+4,25),(x+4,10),(x+8,10)],fill=(237,232,202),width=1)
    for suffix,im in zip(('color','height','roughness','metallic'),images):
        if suffix!='height':im.save(OUT/f'{name}_{suffix}.png',optimize=True)
    height=np.asarray(images[1],dtype=np.float32)/255
    # Periodic central differences keep the tile boundary seamless.
    gx=(np.roll(height,-1,axis=1)-np.roll(height,1,axis=1))*.5
    gy=(np.roll(height,-1,axis=0)-np.roll(height,1,axis=0))*.5
    normal=np.stack((-gx*1.8,gy*1.8,np.ones_like(height)),axis=-1)
    normal/=np.linalg.norm(normal,axis=-1,keepdims=True)
    Image.fromarray(np.uint8(np.clip(normal*.5+.5,0,1)*255)).save(OUT/f'{name}_normal.png',optimize=True)
    return images[0]

tiles=[draw_tile(name) for name in ('paving','masonry','spring_saint','banner','mosaic','vent','walnut','acanthus')]
sheet=Image.new('RGB',(512,256),(32,51,65))
for i,tile in enumerate(tiles):sheet.paste(tile.resize((128,128),Image.Resampling.NEAREST),((i%4)*128,(i//4)*128))
sheet.save(ART/'authored_pixel_materials.png',optimize=True)
print('Authored eight original 32/64px PBR material sets and palette contact sheet')
