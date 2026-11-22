"""Prepare the original alpha furnishings sheet for UV-mapped, lit scenery cards."""
from pathlib import Path
from PIL import Image,ImageFilter
import json
root=Path(__file__).resolve().parents[1];src=Image.open(root/'art/market/furnishings-source.webp').convert('RGBA')
atlas=Image.new('RGBA',(512,256));regions=[]
for n in range(8):
    x=n%4;y=n//4;w,h=src.width//4,src.height//2
    cut=src.crop((x*w,y*h,(x+1)*w,(y+1)*h));alpha=cut.getchannel('A').point(lambda a:255 if a>161 else 0)
    cut.putalpha(alpha);cut=cut.crop(alpha.filter(ImageFilter.MedianFilter(3)).getbbox());cut.thumbnail((120,120),Image.Resampling.LANCZOS)
    cut.putalpha(cut.getchannel('A').point(lambda a:255 if a>132 else 0));ox=x*128+(128-cut.width)//2;oy=y*128+(128-cut.height)//2
    atlas.alpha_composite(cut,(ox,oy));regions.append([ox,oy,cut.width,cut.height])
a=atlas.getchannel('A');atlas=atlas.convert('RGB').quantize(150,dither=Image.Dither.NONE).convert('RGBA');atlas.putalpha(a)
atlas.save(root/'art/market/furnishings-atlas.png',optimize=True)
(root/'art/market/furnishings-uv.json').write_text(json.dumps(regions))
