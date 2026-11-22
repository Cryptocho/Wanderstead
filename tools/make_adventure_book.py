"""Original small-format expedition UI materials; no borrowed game art.
Run: uv run --with pillow python tools/make_adventure_book.py
The art/book sheet is a material/typesetting reference, not a game capture.
"""
from pathlib import Path
import random
import re
from PIL import Image, ImageDraw, ImageFont
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/book'
ART = ROOT / 'art/book'
rng = random.Random(231122)

# The four objects were drawn together for this chapter (OpenAI generate_image).
# Fit each silhouette independently and preserve the wheel and rope's open holes.
objects=Image.open(ART/'tool-plates-source.png').convert('RGBA')
for i,name in enumerate(('buckler','hook','tonic','handwheel')):
    x=(i%2)*objects.width//2; y=(i//2)*objects.height//2
    obj=objects.crop((x,y,x+objects.width//2,y+objects.height//2))
    solid=obj.getchannel('A').point(lambda v:255 if v>150 else 0)
    obj.putalpha(solid); obj=obj.crop(solid.getbbox())
    obj.thumbnail((88,88),Image.Resampling.LANCZOS)
    alpha=obj.getchannel('A').point(lambda v:255 if v>128 else 0)
    obj=obj.convert('RGB').quantize(96,dither=Image.Dither.NONE).convert('RGBA')
    obj.putalpha(alpha); plate_image=Image.new('RGBA',(96,96))
    plate_image.alpha_composite(obj,((96-obj.width)//2,(96-obj.height)//2))
    plate_image.save(OUT/f'{name}-plate.png',optimize=True)


def plate(name, base, rim):
    im = Image.new('RGBA', (96, 96), base)
    d = ImageDraw.Draw(im)
    for _ in range(600):
        x, y = rng.randrange(4, 92), rng.randrange(4, 92)
        delta = rng.randrange(-5, 6)
        d.point((x, y), fill=tuple(max(0, min(255, v + delta)) for v in base[:3]) + (base[3],))
    d.rectangle((1, 1, 94, 94), outline=(17, 22, 20, 240), width=2)
    d.rectangle((3, 3, 92, 92), outline=rim)
    d.line((7, 5, 88, 5), fill=(211, 185, 130, 170))
    d.line((5, 90, 90, 90), fill=(8, 14, 13, 230))
    for x in (7, 88):
        for y in (8, 87):
            d.rectangle((x - 1, y - 1, x + 1, y + 1), fill=(166, 127, 75, 255))
            d.point((x - 1, y - 1), fill=(232, 207, 151, 255))
    im.save(OUT / name)
    return im


glass = plate('expedition-glass.png', (24, 36, 34, 226), (112, 113, 87, 230))
button = plate('expedition-button.png', (65, 48, 33, 255), (163, 122, 69, 255))
font = ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), 24)
small = ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), 20)
ref = Image.new('RGBA', (1600, 900), '#536259')


def nine(im, x, y, w, h, target=None):
    tile = Image.new('RGBA', (w, h))
    for a, b, c, e in [(0, 14, 0, 14), (14, 82, 14, w - 14), (82, 96, w - 14, w)]:
        for f, g, j, k in [(0, 14, 0, 14), (14, 82, 14, h - 14), (82, 96, h - 14, h)]:
            tile.alpha_composite(im.crop((a, f, b, g)).resize((e - c, k - j), Image.Resampling.NEAREST), (c, j))
    (target if target is not None else ref).alpha_composite(tile, (x, y))


def text(x, y, value, face=font, color='#f5e8c8'):
    ImageDraw.Draw(ref).text((x, y), value, font=face, fill=color)


# Read the same root geometry used by Bevy nodes and pointer_over_book.
source = (ROOT / 'src/book.rs').read_text()
literals = [s[1:-1].replace('\\n', '\n') for s in re.findall(r'"(?:[^"\\]|\\.)*"', source)]


def bounds(name):
    raw = re.search(rf'Self::{name}\s*=>\s*\[([^\]]+)\]', source).group(1)
    return tuple(round(1600 - float(v.split('-')[1])) if 'width' in v else round(float(v))
                 for v in raw.split(','))


def face(label, x, y, width):
    nine(button, x, y, width, 48)
    assert font.getlength(label) <= width - 12, (label, width, font.getlength(label))
    text(x + (width - font.getlength(label)) / 2, y + 8, label)


portrait = Image.open(ROOT / 'assets/figures/traveler-portrait.png').convert('RGBA').resize((48, 48), Image.Resampling.LANCZOS)
for area in ['town', 'cistern']:
    ref = Image.new('RGBA', (1600, 900), '#536259')
    for part in ['Place', 'Vitals', 'Notice', 'LeftActions',
                 'TownActions' if area == 'town' else 'DelveActions']:
        nine(glass, *bounds(part))
    x, y, _, _ = bounds('Place')
    text(x + 14, y + 8, '铜壶镇 · 第1天' if area == 'town' else '旧蓄水庭 B1 · 第1天')
    text(x + 14, y + 40, '北水门下井 · 寻回手轮' if area == 'town' else '西侧选器物 · 东侧取手轮', small)
    x, y, _, _ = bounds('Notice')
    notice = ['泉边药师用一颗李子换一瓶药。', '先在旅店旁拾果，再去泉台吧。'] if area == 'town' else [
        '下一拍：守卫→(2, -1)', '离开红格 / Q护卫', '喷口2伤，护卫无效']
    for row, value in enumerate(notice):
        text(x + 14, y + 8 + row * 24, value, small)
    x, y, _, _ = bounds('Journal')
    face('J 旅行手记', x, y, 172)
    x, y, _, _ = bounds('Vitals')
    ref.alpha_composite(portrait, (x + 8, y + 5))
    text(x + 64, y + 9, 'HP 7/10  药×2', small)
    ImageDraw.Draw(ref).rectangle((x + 64, y + 42, x + 308, y + 47), fill='#212c28')
    ImageDraw.Draw(ref).rectangle((x + 64, y + 42, x + 235, y + 47), fill='#77b69c')
    x, y, _, _ = bounds('LeftActions')
    face('E 互动', x + 12, y + 10, 196)
    face('V 调查', x + 216, y + 10, 96)
    if area == 'town':
        x, y, _, _ = bounds('TownActions')
        face('T 投果', x + 12, y + 10, 120)
        face('等候', x + 140, y + 10, 132)
    else:
        x, y, _, _ = bounds('DelveActions')
        face('Q 护卫', x + 12, y + 10, 120)
        face('F 盾击', x + 140, y + 10, 132)
        face('R 药水', x + 12, y + 66, 120)
        face('等候', x + 140, y + 66, 132)
        x, y, w, h = bounds('Mini')
        nine(glass, x, y, w, h)
        text(x + 14, y + 7, 'B1   北 ↑', small)
        grid = (ROOT / 'assets/maps/cistern.grid').read_text().splitlines()
        for row, line in enumerate(grid):
            for col, glyph in enumerate(line):
                seen = (col - 10) ** 2 + (row - 12) ** 2 <= 20
                color = '#7b9187' if seen and glyph != '#' else '#1c302e'
                if col == 10 and row == 12:
                    color = '#7affdb'
                ImageDraw.Draw(ref).rectangle((x + 14 + col * 7, y + 34 + row * 7,
                                               x + 19 + col * 7, y + 39 + row * 7), fill=color)
        text(x + 14, y + 140, '青：你  金：出口', ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), 18))
    text(430, 250, 'MATERIAL / TYPESETTING STUDY — NOT A GAME CAPTURE', small)
    ref.save(ART / f'adventure-{area}-reference.png')
print('Exported four-corner town and cistern sheets using the runtime root geometry.')


def copy(prefix, *values):
    value = next(s for s in literals if s.startswith(prefix))
    for replacement in values:
        value = value.replace('{}', str(replacement), 1)
    return value


def page_body(name, *values):
    pattern = rf'Page::{name}\s*=>\s*format!\("((?:[^"\\]|\\.)*)"'
    value = re.search(pattern, source).group(1).replace('\\n', '\n')
    for replacement in values:
        value = value.replace('{}', str(replacement), 1)
    return value


pages = Image.new('RGBA', (1240, 1360), '#37453e')
paper = Image.open(OUT / 'folio.png').convert('RGBA').resize((620, 650))
title_font = ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), 28)


def page(x, y, title, summary, body, confirm):
    pages.alpha_composite(paper, (x, y))
    d = ImageDraw.Draw(pages)
    d.text((x + 64, y + 32), title, font=title_font, fill='#403026')
    for offset, value, color in [(98, summary, '#6d543b'), (226, body, '#403026')]:
        for line, text in enumerate(value.split('\n')):
            assert font.getlength(text) <= 490, (text, font.getlength(text))
            d.text((x + 64, y + offset + line * 28), text, font=font, fill=color)
    nine(button, x + 306, y + 574, 248, 48, pages)
    d.text((x + 326, y + 582), confirm, font=font, fill='#f5e8c8')


page(0, 40, '工具箱 · 选一套器物',
     copy('旧箱子里躺着一面圆盾，', '圆盾'), '', 'E  带上圆盾')
for y, prefix, name in [(220, '圆盾 · 守住窄路','buckler'), (360, '钩索 · 隔格先手','hook')]:
    nine(button, 64, 40 + y, 490, 126, pages)
    pages.alpha_composite(Image.open(OUT/f'{name}-plate.png').resize((86,86),Image.Resampling.NEAREST),(80,40+y+19))
    for line, value in enumerate(copy(prefix).split('\n')):
        ImageDraw.Draw(pages).text((183, 40 + y + 19 + line * 28), value, font=font, fill='#f5e8c8')
page(620, 40, '灯火接你回家', copy('回程的灯提到了你身边。'),
     page_body('Down', 2, 0, 10, 8), 'E  回到铜壶镇')
page(0, 700, '第一股泉水', copy('起初只有几滴水，'),
     page_body('Homecoming', 3, 4, 8, 1, '钩索'), 'E  继续旅行')
pages.alpha_composite(Image.open(OUT/'handwheel-plate.png').resize((83,83),Image.Resampling.NEAREST),(468,799))
page(620, 700, '旅行手记 · 出征',
     '所在：旧泉回水廊 B1\n寻找东侧的青铜手轮\n西侧工具箱可选一套器物',
     copy('泉水：', '仍待修复', '尚在回水廊东侧', 8, 10, 2, '圆盾', 2, 8, 3, 1, 1),
     '下一页')
ImageDraw.Draw(pages).text((18, 8), 'TYPESETTING SPECIMENS / NOT GAME CAPTURES', font=small, fill='#f5e8c8')
pages.save(ART / 'adventure-pages-reference.png')
print('Exported modal typography specimens using actual Rust UI copy.')
