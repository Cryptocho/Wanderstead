"""Compact roadside stationery from this project's existing original textures.
Run: uv run --with pillow python tools/make_book_road.py
The reference is an authoring sheet, not a game capture.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/book'; ART=ROOT/'art/book'
paper=Image.open(OUT/'folio.png').convert('RGBA').crop((94,110,564,332))

def sheet(w,h):
 im=Image.new('RGBA',(w,h),'#654432')
 for x in range(3,w-3,paper.width):
  for y in range(3,h-7,paper.height):
   tile=paper.crop((0,0,min(paper.width,w-3-x),min(paper.height,h-7-y)))
   im.alpha_composite(tile,(x,y))
 d=ImageDraw.Draw(im)
 d.rectangle((1,1,w-2,h-2),outline='#bb9966')
 d.line((4,4,w-5,4),fill='#f0dfae')
 d.line((4,h-6,w-5,h-6),fill='#a78354')
 for x in (7,w-8):
  d.ellipse((x-2,6,x+2,10),fill='#b68d51',outline='#644830')
  d.point((x-1,7),fill='#ead299')
 return im
rail=sheet(660,94)
d=ImageDraw.Draw(rail)
d.line((18,36,642,36),fill='#b99d6d')
rail.save(OUT/'road-strip.png')
tag=sheet(470,222)
d=ImageDraw.Draw(tag)
d.rectangle((3,3,13,214),fill='#72503b')
for y in range(12,208,14):d.line((5,y,10,y+4),fill='#caab76')
d.line((24,46,446,46),fill='#b99d6d')
tag.save(OUT/'field-tag.png')

font=ImageFont.truetype(str(OUT/'FusionPixel.ttf'),24)
button=Image.open(OUT/'button.png').convert('RGBA')
ref=Image.new('RGBA',(1600,900),'#566753')
ref.alpha_composite(rail,(24,788));ref.alpha_composite(tag,(24,554))
d=ImageDraw.Draw(ref)
d.text((48,566),'往墙那边，投一颗',font=font,fill='#403026')
for y,s in [(606,'手里 2 颗，三步内可越墙。'),(634,'方向键 / 点地面选落点。'),(662,'落点：东 4，南 -2')]:d.text((48,y),s,font=font,fill='#403026')
d.text((44,794),'壶下的桌上摆着早餐。',font=font,fill='#403026')

def draw_button(x,y,w,s):
 # Repeat the same 14px nine-slice borders used by the runtime button.
 skin=Image.new('RGBA',(w,46))
 for sx,sw,dx,dw in [(0,14,0,14),(14,68,14,w-28),(82,14,w-14,14)]:
  skin.alpha_composite(button.crop((sx,0,sx+sw,48)).resize((dw,46),Image.Resampling.NEAREST),(dx,0))
 ref.alpha_composite(skin,(x,y))
 box=font.getbbox(s); tx=x+(w-font.getlength(s))/2; ty=y+(46-(box[3]-box[1]))/2-box[1]
 ImageDraw.Draw(ref).text((tx,ty),s,font=font,fill='#f3dfb5')
 assert font.getlength(s)<=w-16,(s,w,font.getlength(s))
for x,w,s in [(44,188,'E  拾起李子'),(238,94,'T 投果'),(338,94,'V 观察'),(438,94,'J 手记'),(538,126,'等候')]:draw_button(x,828,w,s)
draw_button(48,712,226,'E  投出一颗');draw_button(286,712,112,'取消')
ImageDraw.Draw(ref).text((24,148),'AUTHORING REFERENCE / NOT A GAME CAPTURE',font=ImageFont.truetype(str(OUT/'FusionPixel.ttf'),18),fill='#ede6ce')
ref.save(ART/'road-reference.png')
print('Exported 660x94 roadside strip, 470x222 field tag, and art/book/road-reference.png')
print('24px label widths fit all button faces (>=8px horizontal margin per side).')
