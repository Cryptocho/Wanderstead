"""Bake original foliage/rock paintings to small hard-alpha, palette-limited surfaces.
Source: art/market/spring-verge-source.png (original generate_image for this scene).
The two crown lobes are layered in Blender; the stones are low cards at real banks.
"""
from pathlib import Path
from PIL import Image,ImageFilter
root=Path(__file__).resolve().parents[1];out=root/'art/market'
src=Image.open(out/'spring-verge-source.png').convert('RGBA');w,h=src.size
for n,name in enumerate(('verge-leaves','verge-blossoms','verge-stone','verge-reedstone')):
    x=n%2*w//2;y=n//2*h//2
    tile=src.crop((x,y,x+w//2,y+h//2));a=tile.getchannel('A').point(lambda v:255 if v>=200 else 0).filter(ImageFilter.MedianFilter(3));tile.putalpha(a)
    tile=tile.crop(a.getbbox());tile.thumbnail((76,76),Image.Resampling.LANCZOS)
    a=tile.getchannel('A').point(lambda v:255 if v>141 else 0)
    col=tile.convert('RGB').quantize(32,method=Image.Quantize.MEDIANCUT,dither=Image.Dither.NONE).convert('RGBA');col.putalpha(a)
    if n<2:
        # Baseline-aligned lobe silhouette with gutters for the crossed-face shells.
        plate=Image.new('RGBA',(80,74));plate.alpha_composite(col,((80-col.width)//2,(74-col.height)//2));col=plate
    col.save(out/(name+'.png'),optimize=True)
