"""Bake six seamless original late-morning sky faces for Bevy (Pillow only)."""
import math
from pathlib import Path
from PIL import Image

size = 192
# Azimuth measured from north, elevation; distant banks have flat, hazy bases.
banks = [(-2.6, .11, .32), (-1.48, .22, .27), (-.49, .115, .33), (.10, .23, .18), (.50, .125, .32), (1.08, .33, .15), (2.08, .16, .39)]
lobes = [(-.72, .21, .43, .31), (-.36, .42, .44, .50), (-.01, .57, .39, .65), (.29, .39, .45, .46), (.63, .17, .42, .30)]

def mix(a,b,t): return tuple(x+(y-x)*t for x,y in zip(a,b))
def pixel(x,y,z):
    length=math.sqrt(x*x+y*y+z*z); y/=length
    az=math.atan2(x,-z); el=math.asin(y)
    t=min(1., max(0., y*1.55))**.65
    rgb=mix((215,225,213),(84,148,202),t)
    for angle,base,width in banks:
        u=((az-angle+math.pi)%(2*math.pi)-math.pi)/width
        v=(el-base)/(width*.34)
        if abs(u)>1.1 or not -.12<v<1.3: continue
        rim=min(((u-cx)/rx)**2+((v-cy)/ry)**2 for cx,cy,rx,ry in lobes)
        if rim>=1:continue
        # Broad, deliberately quiet planes of colour under the lit cloud edge.
        cover=min(1.,(1.-rim)*11.,(v+.12)*12.)
        wash=mix((204,213,218),(250,243,216),max(0,min(1,(v+.23)*1.3)))
        rgb=mix(rgb,wash,max(0.,cover)*.88)
    return tuple(round(c) for c in rgb)

out=Image.new('RGB',(size,size*6)); px=out.load()
for face in range(6):
    for row in range(size):
        v=2*(row+.5)/size-1
        for col in range(size):
            u=2*(col+.5)/size-1
            d=((1,-v,-u),(-1,-v,u),(u,1,v),(u,-1,-v),(u,-v,1),(-u,-v,-1))[face]
            px[col,row+face*size]=pixel(*d)
out.save(Path(__file__).resolve().parents[1]/'assets/inks/morning-sky.png', optimize=True)
