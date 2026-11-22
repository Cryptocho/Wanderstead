"""Original Wanderstead book assets. Run: uv run --with pillow python tools/make_book.py.
No game capture is produced: art/book/road-reference.png is a typesetting reference.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import random, math, zipfile, runpy
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/book'; ART=ROOT/'art/book'
OUT.mkdir(parents=True,exist_ok=True); ART.mkdir(parents=True,exist_ok=True)
rng=random.Random(49018)

def noise(w,h,base,amount=4):
 im=Image.new('RGBA',(w,h)); p=im.load()
 for y in range(h):
  for x in range(w):
   n=rng.uniform(-amount,amount)
   p[x,y]=tuple(max(0,min(255,int(c+n))) for c in base)+(255,)
 return im

def paper(w,h):
 im=noise(w,h,(226,208,165),4); d=ImageDraw.Draw(im)
 # Beaten pulp, long visible fibres and faint uneven watermarks.
 for _ in range(w*h//70):
  x=rng.randrange(w); y=rng.randrange(h); l=rng.randrange(2,13)
  d.line((x,y,x+l,y+rng.choice([-1,0,1])),fill=rng.choice([(205,186,143),(238,224,190),(217,196,155)]))
 for inset in range(9):
  c=(145+inset*8,111+inset*10,69+inset*10)
  d.rectangle((inset,inset,w-1-inset,h-1-inset),outline=c)
 d.line((12,11,w-13,11),fill=(247,232,191),width=2)
 return im

def metal(d,box):
 x,y,X,Y=box; d.rounded_rectangle(box,radius=3,fill='#a06b39',outline='#4c3427',width=2)
 d.line((x+3,y+2,X-3,y+2),fill='#e1ba72',width=2)
 for cx,cy in [(x+4,y+5),(X-4,Y-5)]:
  d.ellipse((cx-2,cy-2,cx+2,cy+2),fill='#523b2b');d.point((cx,cy-1),fill='#efd193')

def wood(w,h):
 im=noise(w,h,(91,57,36),5); d=ImageDraw.Draw(im)
 for y in range(4,h-3,3):
  points=[(x,y+int(math.sin(x/47+y)*1.9)) for x in range(4,w-4)]
  d.line(points,fill=rng.choice(['#704b30','#462e23','#795337','#583922']))
 for x,y in [(w*.24,h*.46),(w*.78,h*.74)]:
  for a in [10,16,23]:d.ellipse((x-a,y-a*.22,x+a,y+a*.22),outline='#462e23')
 d.rounded_rectangle((2,2,w-3,h-3),radius=7,outline='#33291e',width=4)
 d.line((9,7,w-10,7),fill='#b48a51',width=2)
 for x in [12,w-26]:metal(d,(x,12,x+14,h-13))
 return im

# Native-sized fixed panels preserve their grain instead of stretching a tiny patch.
wood(342,90).save(OUT/'inn-sign.png')
stock=paper(300,90); d=ImageDraw.Draw(stock);metal(d,(278,10,290,79));stock.save(OUT/'inventory.png')
book=noise(620,610,(72,43,36),5); d=ImageDraw.Draw(book)
d.rounded_rectangle((1,1,618,608),radius=12,outline='#2c2520',width=5)
# Three exposed page edges, leather spine, saddle stitching and copper clasp.
for off in [0,3,6]:book.alpha_composite(paper(553,576),(48+off,13+off))
d=ImageDraw.Draw(book)
for x in [10,15,27,34]:d.line((x,17,x,591),fill='#a87b4c' if x!=27 else '#36271f',width=2)
for y in range(28,589,15):d.line((18,y,25,y+5),fill='#cfaa72',width=2)
for y in [81,501]:metal(d,(3,y,42,y+26))
metal(d,(593,271,619,317));d.ellipse((600,286,609,296),fill='#392c24',outline='#d4b06e')
# Printed margin, foxing marks, ruled chapter separator.
d.line((87,44,87,565),fill='#c7a879')
for y in [89,257,521]:d.line((104,y,565,y),fill='#b99d6d')
for _ in range(110):
 x=rng.choice([rng.randrange(57,81),rng.randrange(578,602)]);y=rng.randrange(24,575)
 d.point((x,y),fill='#bba073')
book.save(OUT/'folio.png')
button=wood(96,48);d=ImageDraw.Draw(button)
d.rounded_rectangle((6,6,89,41),radius=3,outline='#aa814d',width=1)
button.save(OUT/'button.png')

# Woodcut-like illustrations, drawn with a constrained warm palette.
def plum(d,x,y,s=1):
 d.line((x,y-10*s,x+4*s,y-23*s),fill='#674a2d',width=max(2,int(3*s)))
 d.ellipse((x+3*s,y-24*s,x+20*s,y-17*s),fill='#71844d',outline='#354a35')
 d.ellipse((x-18*s,y-13*s,x+18*s,y+24*s),fill='#664963',outline='#49384d',width=max(2,int(2*s)))
 d.arc((x-12*s,y-9*s,x+12*s,y+19*s),95,250,fill='#a9798c',width=max(2,int(3*s)))
 d.line((x+4*s,y-10*s,x+5*s,y+20*s),fill='#523949',width=2)
icon=Image.new('RGBA',(64,64));plum(ImageDraw.Draw(icon),29,32,.85);icon.save(OUT/'plum.png')
ill=Image.new('RGBA',(474,142));d=ImageDraw.Draw(ill)
# Porcelain bowl, spoon, linen, fruit. Ink hatching remains legible at 1:1.
d.polygon([(44,111),(257,100),(292,136),(23,136)],fill='#c4b48c',outline='#8d7755')
for x in range(42,270,8):d.line((x,116,x+17,132),fill='#ae9c76')
d.ellipse((60,62,249,121),fill='#8c7357')
d.pieslice((62,32,247,126),0,180,fill='#d8d3b7',outline='#6c6757',width=3)
d.ellipse((62,30,247,84),fill='#f0e3bf',outline='#716956',width=3)
d.ellipse((72,37,237,75),fill='#fff0cd',outline='#c5b58f',width=2)
plain=ill.copy()
pd=ImageDraw.Draw(plain)
pd.line((235,40,324,14),fill='#8a6b42',width=7)
pd.ellipse((217,33,245,49),fill='#b28f5d',outline='#705639',width=2)
plain.save(OUT/'plain-breakfast.png')
for x,y in [(123,52),(158,58),(181,48)]:
 d.polygon([(x-12,y),(x+4,y-8),(x+15,y+3),(x,y+10)],fill='#95465a',outline='#6b3d4a')
 d.line((x-4,y,x+7,y+3),fill='#d2908d',width=3)
d.line((235,40,324,14),fill='#8a6b42',width=7);d.ellipse((217,33,245,49),fill='#b28f5d',outline='#705639',width=2)
plum(d,345,83,1.1);plum(d,393,94,.8)
d.line((310,128,440,128),fill='#b5996c',width=2)
ill.save(OUT/'breakfast.png')
# Small field diagram: fruit flies over the wall, the creature must walk around.
field=Image.new('RGBA',(474,142));d=ImageDraw.Draw(field)
for x in range(5,470,26):d.line((x,126,x+10,126),fill='#b59c70')
for y in [18,40,62,84]:
 d.rectangle((200,y,235,y+18),fill='#a99d7b',outline='#72664c',width=2)
 d.line((203,y+2,231,y+2),fill='#dbceaa',width=2)
plum(d,320,54,.75)
for t in range(0,101,6):
 x=120+t*2;y=70-math.sin(t/100*math.pi)*55
 d.line((x,y,x+7,y),fill='#865c45',width=2)
d.polygon([(314,53),(305,43),(303,59)],fill='#865c45')
d.ellipse((101,62,126,88),fill='#96704e',outline='#5f4934',width=2)
d.polygon([(96,86),(131,86),(137,112),(91,112)],fill='#925e4c',outline='#5f4934')
d.ellipse((300,88,345,117),fill='#809473',outline='#526a52',width=2)
d.ellipse((316,95,327,109),fill='#76556f')
d.line((318,119,275,132,181,132,152,113),fill='#8c8863',width=2)
field.save(OUT/'field-notes.png')
# Inn crest: suspended copper kettle under a little roof.
im=Image.new('RGBA',(64,64));d=ImageDraw.Draw(im)
d.line((9,19,30,7,55,19),fill='#dec18a',width=3);d.line((31,10,31,29),fill='#bb9c68',width=2)
d.arc((19,18,45,46),180,360,fill='#e4bc7a',width=3)
d.ellipse((16,30,48,54),fill='#af7446',outline='#e7c38b',width=2)
d.polygon([(45,37),(58,29),(53,44),(46,47)],fill='#bb8751',outline='#e7c38b')
d.line((22,35,20,43,24,47),fill='#e2ac65',width=2);im.save(OUT/'kettle.png')

# Extract only the unmodified Simplified-Chinese face; retain every bundled notice.
zpath=ART/'source/fusion-12.zip'
if zpath.exists():
 with zipfile.ZipFile(zpath) as z:
  (OUT/'FusionPixel.ttf').write_bytes(z.read('fusion-pixel-12px-proportional-zh_hans.ttf'))
  for n in z.namelist():
   if not n.endswith('.ttf') and not n.endswith('/'):
    target=OUT/'licenses'/n;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(z.read(n))
# Export the current compact rail and field tag after the unchanged binding.
runpy.run_path(str(ROOT/'tools/make_book_road.py'),run_name='__main__')
print('Original book textures exported to assets/book')
