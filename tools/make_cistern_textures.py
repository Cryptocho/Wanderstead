#!/usr/bin/env python3
"""Prepare original generated material scans as glTF-ready PBR maps (Pillow+NumPy)."""
from pathlib import Path
from PIL import Image, ImageFilter
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
ART=ROOT/'art/cistern'
OUT=ROOT/'assets/cistern'
OUT.mkdir(parents=True,exist_ok=True)
for source,names in [('spring_material_atlas.png',['paving','masonry','spring_saint','banner']),('ornament_material_atlas.png',['mosaic','vent','walnut','acanthus'])]:
    im=Image.open(ART/'source'/source).convert('RGB')
    for i,name in enumerate(names):
        x=(i%2)*512;y=(i//2)*512
        tile=im.crop((x+2,y+2,x+510,y+510)).resize((512,512),Image.Resampling.LANCZOS)
        tile.save(OUT/f'{name}_color.png',optimize=True)
        rgb=np.asarray(tile,dtype=np.float32)/255
        h=np.asarray(tile.convert('L').filter(ImageFilter.GaussianBlur(1.0)),dtype=np.float32)/255
        gy,gx=np.gradient(h)
        strength=4 if name in ('banner','walnut') else 7
        n=np.stack((-gx*strength,gy*strength,np.ones_like(h)),axis=-1)
        n/=np.linalg.norm(n,axis=-1,keepdims=True)
        Image.fromarray(np.uint8(np.clip(n*.5+.5,0,1)*255)).save(OUT/f'{name}_normal.png',optimize=True)
        rough=np.clip(.66+(h-.5)*.25,.4,.91)
        if name in ('paving','mosaic'):rough=np.clip(.39+h*.30,.38,.74)
        if name=='vent':rough=np.clip(.37+h*.35,.36,.74)
        Image.fromarray(np.uint8(rough*255)).save(OUT/f'{name}_roughness.png',optimize=True)
print('Prepared eight original 512px PBR material sets')
