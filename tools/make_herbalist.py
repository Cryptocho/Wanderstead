"""Original generated herb-seller strip -> four foot-aligned pixel frames and journal portrait.
Input: art/market/herbalist-source.webp, generated for this game with OpenAI gpt-image-2.
The generation is transparent; threshold its matte before resampling and quantize all poses together.
"""
from pathlib import Path
from PIL import Image, ImageFilter
root=Path(__file__).resolve().parents[1]
src=Image.open(root/'art/market/herbalist-source.webp').convert('RGBA')
frames=[]
for i in range(4):
    im=src.crop((i*src.width//4,0,(i+1)*src.width//4,src.height))
    a=im.getchannel('A').point(lambda v:255 if v>173 else 0)
    box=a.filter(ImageFilter.MedianFilter(5)).getbbox();im.putalpha(a);frames.append(im.crop(box))
scale=108/max(f.height for f in frames)
atlas=Image.new('RGBA',(4*96,128))
for i,f in enumerate(frames):
    f=f.resize((round(f.width*scale),round(f.height*scale)),Image.Resampling.LANCZOS)
    f.putalpha(f.getchannel('A').point(lambda v:255 if v>=140 else 0));atlas.alpha_composite(f,(i*96+(96-f.width)//2,126-f.height))
a=atlas.getchannel('A');atlas=atlas.convert('RGB').quantize(72,dither=Image.Dither.NONE).convert('RGBA');atlas.putalpha(a)
atlas.save(root/'assets/market/herbalist.png',optimize=True)
portrait=atlas.crop((0,10,96,94));portrait.save(root/'assets/market/herbalist-face.png',optimize=True)
