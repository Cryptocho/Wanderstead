"""Original compact Wanderstead stationery and hand-pixelled command icons.
Run: uv run --with pillow python tools/make_adventure_book.py
All reference sheets are labelled authoring specimens, never game captures.
The HUD sheet uses the exact HudPart rectangles and font defaults in book.rs.
"""
from pathlib import Path
import json
import math
import random
import re
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/book'
ART = ROOT / 'art/book'
SOURCE = (ROOT / 'src/book.rs').read_text()
rng = random.Random(231122)
INK = '#403026'
IVORY = '#f5e8c8'
BRASS = '#c49b55'
DARK = '#392c25'

# The four original chapter objects share one retained generate_image source.
objects = Image.open(ART / 'tool-plates-source.png').convert('RGBA')
for i, name in enumerate(('buckler', 'hook', 'tonic', 'handwheel')):
    x, y = (i % 2) * objects.width // 2, (i // 2) * objects.height // 2
    obj = objects.crop((x, y, x + objects.width // 2, y + objects.height // 2))
    solid = obj.getchannel('A').point(lambda v: 255 if v > 150 else 0)
    obj.putalpha(solid)
    obj = obj.crop(solid.getbbox())
    obj.thumbnail((88, 88), Image.Resampling.LANCZOS)
    alpha = obj.getchannel('A').point(lambda v: 255 if v > 128 else 0)
    obj = obj.convert('RGB').quantize(96, dither=Image.Dither.NONE).convert('RGBA')
    obj.putalpha(alpha)
    plate_image = Image.new('RGBA', (96, 96))
    plate_image.alpha_composite(obj, ((96 - obj.width) // 2, (96 - obj.height) // 2))
    plate_image.save(OUT / f'{name}-plate.png', optimize=True)


def plate(name, base, rim):
    """Nine-slice: quiet oak endgrain, one-pixel brass seams and rivets."""
    im = Image.new('RGBA', (96, 96), base)
    d = ImageDraw.Draw(im)
    for y in range(8, 90, 5):
        for start in range(8, 90, 16):
            x = start + rng.randrange(-3, 4)
            delta = rng.choice((-3, -2, 2, 3))
            color = tuple(max(0, min(255, v + delta)) for v in base[:3]) + (base[3],)
            d.line((x, y, min(88, x + rng.randrange(4, 15)), y), fill=color)
    d.rectangle((1, 1, 94, 94), outline=(26, 23, 18, 245), width=2)
    d.rectangle((3, 3, 92, 92), outline=rim)
    d.line((7, 5, 88, 5), fill=(224, 193, 133, 180))
    d.line((5, 90, 90, 90), fill=(13, 15, 12, 230))
    for x in (7, 88):
        for y in (8, 87):
            d.rectangle((x - 1, y - 1, x + 1, y + 1), fill=(166, 127, 75, 255))
            d.point((x - 1, y - 1), fill=(242, 218, 160, 255))
    im.save(OUT / name, optimize=True)
    return im


glass = plate('expedition-glass.png', (43, 38, 28, 232), (132, 112, 75, 230))
button = plate('expedition-button.png', (80, 60, 37, 255), (179, 142, 83, 255))


def icon(name):
    """Every coordinate is authored on a transparent 24px grid, no borrowed art."""
    im = Image.new('RGBA', (24, 24))
    d = ImageDraw.Draw(im)
    if name == 'context':
        d.polygon([(5, 13), (7, 11), (9, 12), (9, 5), (11, 3), (13, 5),
                   (13, 10), (18, 10), (20, 13), (18, 20), (10, 21)], fill=DARK)
        d.polygon([(7, 13), (9, 14), (10, 7), (11, 5), (12, 12), (17, 12),
                   (18, 14), (16, 19), (11, 19)], fill='#d6b887')
        d.line((11, 7, 11, 13), fill='#fff0c4')
        d.rectangle((10, 20, 16, 22), fill='#72968d')
        d.line((17, 3, 17, 7), fill=BRASS)
        d.line((15, 5, 19, 5), fill=BRASS)
    elif name == 'look':
        d.line((14, 14, 21, 21), fill=DARK, width=6)
        d.line((15, 15, 20, 20), fill='#b7864e', width=3)
        d.ellipse((2, 2, 17, 17), fill=DARK)
        d.ellipse((3, 3, 16, 16), fill=BRASS)
        d.ellipse((5, 5, 14, 14), fill='#719a9d')
        d.line((6, 11, 11, 6), fill='#d9eee1', width=2)
        d.point((13, 9), fill='#f7f0c9')
    elif name == 'journal':
        d.polygon([(3, 4), (7, 2), (20, 4), (20, 21), (6, 20), (3, 22)], fill=DARK)
        d.rectangle((6, 4, 18, 20), fill='#976644')
        d.rectangle((9, 5, 18, 18), fill='#eddaa7')
        d.line((7, 4, 7, 20), fill='#c5975f')
        for y in (8, 11, 14):
            d.line((11, y, 16, y), fill='#a68c64')
        d.rectangle((16, 12, 21, 15), fill=BRASS)
        d.point((18, 13), fill='#fff0c4')
        d.line((4, 6, 4, 19), fill='#d6b17c')
    elif name == 'fruit':
        d.line((12, 5, 14, 1), fill='#a68c57', width=2)
        d.polygon([(13, 5), (15, 1), (21, 2), (17, 6)], fill='#6f9957')
        d.polygon([(5, 7), (10, 5), (13, 6), (17, 5), (21, 9),
                   (20, 17), (16, 21), (8, 21), (3, 16), (3, 10)], fill=DARK)
        d.ellipse((4, 7, 19, 20), fill='#8e526d')
        d.ellipse((5, 7, 12, 15), fill='#bd7890')
        d.rectangle((7, 8, 9, 10), fill='#f1c7bd')
        d.line((14, 8, 15, 18), fill='#663f60')
    elif name == 'wait':
        d.rectangle((5, 2, 19, 4), fill=DARK)
        d.rectangle((5, 20, 19, 22), fill=DARK)
        d.line((7, 5, 7, 9, 12, 13, 7, 17, 7, 19), fill=BRASS, width=2)
        d.line((17, 5, 17, 9, 12, 13, 17, 17, 17, 19), fill='#ecd09b', width=2)
        d.polygon([(9, 7), (15, 7), (12, 11)], fill='#c7dbcf')
        d.polygon([(9, 18), (12, 15), (15, 18)], fill='#ecd09b')
        d.rectangle((6, 2, 18, 3), fill='#e6bd74')
        d.rectangle((6, 20, 18, 21), fill='#a87943')
    elif name == 'brace':
        d.polygon([(3, 4), (12, 1), (21, 4), (20, 15), (17, 20),
                   (12, 23), (7, 20), (4, 15)], fill=DARK)
        d.polygon([(5, 5), (12, 3), (19, 5), (18, 15), (15, 19),
                   (12, 21), (9, 19), (6, 15)], fill=BRASS)
        d.polygon([(7, 7), (12, 5), (17, 7), (16, 15), (12, 18), (8, 15)], fill='#768b83')
        d.line((12, 6, 12, 17), fill='#dce0ba', width=2)
        d.line((8, 11, 16, 11), fill='#dce0ba', width=2)
        d.point((7, 5), fill='#fff0c4')
    elif name == 'skill':
        d.polygon([(6, 14), (17, 2), (22, 1), (21, 6), (10, 18)], fill=DARK)
        d.polygon([(8, 14), (18, 4), (20, 3), (19, 6), (10, 16)], fill='#d3ded0')
        d.line((10, 13, 19, 4), fill='#fff5d0')
        d.line((5, 13, 12, 20), fill=BRASS, width=3)
        d.line((4, 21, 8, 17), fill='#ac7450', width=3)
        d.rectangle((2, 20, 4, 22), fill='#e6bd74')
        d.line((3, 5, 3, 9), fill='#e6bd74')
        d.line((1, 7, 5, 7), fill='#e6bd74')
    elif name == 'drink':
        d.rectangle((9, 1, 15, 4), fill=DARK)
        d.rectangle((10, 1, 14, 3), fill='#bc8a52')
        d.polygon([(8, 5), (16, 5), (16, 10), (20, 15), (20, 20),
                   (17, 23), (7, 23), (4, 20), (4, 15), (8, 10)], fill=DARK)
        d.polygon([(10, 6), (14, 6), (14, 11), (18, 16), (18, 20),
                   (16, 21), (8, 21), (6, 19), (6, 16), (10, 11)], fill='#9cb8ad')
        d.polygon([(8, 15), (16, 15), (17, 19), (15, 20), (9, 20), (7, 18)], fill='#bd6261')
        d.line((8, 16, 8, 18), fill='#ffddbc', width=2)
        d.line((9, 7, 15, 7), fill='#eadbb1')
    elif name == 'hook':
        d.ellipse((2, 11, 15, 23), fill=DARK)
        d.ellipse((3, 12, 14, 22), outline='#c7a568', width=3)
        d.ellipse((6, 15, 11, 20), outline='#f1d79a')
        d.polygon([(8, 14), (13, 6), (12, 2), (16, 3), (18, 1),
                   (20, 2), (20, 6), (23, 8), (20, 10), (17, 9),
                   (12, 17)], fill=DARK)
        d.line((10, 14, 17, 5), fill='#9dafa6', width=3)
        d.line((12, 11, 17, 5), fill='#e1e1bc')
        d.polygon([(14, 3), (17, 5), (19, 2), (18, 7), (21, 8),
                   (17, 8), (16, 7)], fill='#c0c6b0')
        d.line((8, 13, 12, 16), fill='#dfbf80', width=2)
    im.save(OUT / f'icon-{name}.png', optimize=True)
    return im


NAMES = ('context', 'look', 'journal', 'fruit', 'wait', 'brace', 'skill', 'drink', 'hook')
icons = {name: icon(name) for name in NAMES}
compass = Image.new('RGBA', (48, 48))
d = ImageDraw.Draw(compass)
d.ellipse((4, 4, 43, 43), outline=DARK, width=3)
d.ellipse((5, 5, 42, 42), outline=BRASS)
d.ellipse((9, 9, 38, 38), outline='#927b50')
for angle in range(0, 360, 45):
    a = math.radians(angle)
    d.line((24 + round(math.sin(a) * 15), 24 + round(math.cos(a) * 15),
            24 + round(math.sin(a) * 18), 24 + round(math.cos(a) * 18)), fill='#e5cc92')
d.polygon([(24, 6), (29, 26), (24, 23), (19, 26)], fill='#f7d79a')
d.polygon([(24, 41), (29, 22), (24, 25), (19, 22)], fill='#93856e')
d.polygon([(8, 24), (26, 20), (24, 24), (26, 28)], fill='#b79a69')
d.polygon([(40, 24), (22, 20), (24, 24), (22, 28)], fill='#d6be88')
d.rectangle((22, 22, 26, 26), fill=DARK)
d.rectangle((23, 23, 25, 25), fill='#f7d79a')
compass.save(OUT / 'compass.png', optimize=True)

sizes = {key: float(re.search(rf'{key}:\s*([0-9.]+)', SOURCE).group(1))
         for key in ('hud', 'reading', 'heading')}
font = ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), round(sizes['hud']))
reading = ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), round(sizes['reading']))
heading = ImageFont.truetype(str(OUT / 'FusionPixel.ttf'), round(sizes['heading']))


def bounds(name):
    raw = re.search(rf'Self::{name}\s*=>\s*\[([^\]]+)\]', SOURCE).group(1)
    return tuple(round(1600 - float(v.split('-')[1])) if 'width' in v else round(float(v))
                 for v in raw.split(','))


def nine(target, im, x, y, w, h):
    tile = Image.new('RGBA', (w, h))
    for a, b, c, e in [(0, 14, 0, 14), (14, 82, 14, w - 14), (82, 96, w - 14, w)]:
        for f, g, j, k in [(0, 14, 0, 14), (14, 82, 14, h - 14), (82, 96, h - 14, h)]:
            tile.alpha_composite(im.crop((a, f, b, g)).resize((e - c, k - j), Image.Resampling.NEAREST), (c, j))
    target.alpha_composite(tile, (x, y))


def text(target, x, y, value, face=font, color=IVORY):
    ImageDraw.Draw(target).text((x, y), value, font=face, fill=color, spacing=round(face.size * .3))


def face(target, label, name, x, y, width, height=36, face_font=font):
    nine(target, button, x, y, width, height)
    iw = 29 if name else 0
    label_width = face_font.getlength(label)
    assert label_width + iw <= width - 12, (label, width, label_width + iw)
    left = round(x + (width - label_width - iw) / 2)
    if name:
        target.alpha_composite(icons[name], (left, y + (height - 24) // 2))
    text(target, left + iw, y + (height - face_font.size) // 2 - 2, label, face_font)


for area in ('town', 'cistern'):
    ref = Image.new('RGBA', (1600, 900), '#66746a')
    text(ref, 1000, 10, 'AUTHORING SPECIMEN / NOT A GAME CAPTURE')
    for part in ('Vitals', 'Mini'):
        nine(ref, glass, *bounds(part))
    x, y, _, _ = bounds('Place')
    text(ref, x, y, '第1天  铜壶镇  ·  北水门下井 · 寻回手轮' if area == 'town' else
         '第1天  旧泉回水廊 B1  ·  东侧取手轮 · 留意红格')
    x, y, _, _ = bounds('Compass')
    ref.alpha_composite(compass, (x, y + 10))
    text(ref, x + 20, y, 'N')
    text(ref, x + 54, y + 28, '镇 (0,4)' if area == 'town' else 'B1 (0,4)')
    message=('泉边药师用一颗李子换一瓶药。\n你从旅店旁拾起一颗熟李子。' if area=='town' else
             '危险 · 下一拍 守卫→(2,-1)  离开红格 / Q护卫；喷口2伤无视护卫。\n你沿着北侧石路走了一步。')
    x, _, _, _=bounds('Notice');h=math.ceil(2*font.size*1.3+10);y=900-96-h
    w=max(146,math.ceil(max(font.getlength(line) for line in message.splitlines())+20))
    nine(ref,glass,x,y,w,h)
    text(ref,x+10,y+5,message,color=IVORY if area=='town' else '#ffa86e')
    x, y, _, _ = bounds('Vitals')
    portrait = Image.open(ROOT / 'assets/figures/traveler-portrait.png').convert('RGBA').resize((32, 32), Image.Resampling.NEAREST)
    ref.alpha_composite(portrait, (x + 4, y))
    text(ref, x + 42, y + 1, 'HP 7/10  药2  果3  铜4')
    ImageDraw.Draw(ref).rectangle((x + 42, y + 25, x + 302, y + 28), fill='#292119')
    ImageDraw.Draw(ref).rectangle((x + 42, y + 25, x + 224, y + 28), fill='#6bba99')
    x, y, _, _ = bounds('Guide')
    text(ref, x, y, '斜走：同时按两个方向键，或按小键盘。J 翻手记。')
    x, y, _, _ = bounds('LeftActions')
    face(ref, 'E 互动', 'context', x, y, 160)
    face(ref, 'V 调查', 'look', x + 160, y, 102)
    x, y, _, _ = bounds('Journal')
    face(ref, 'J 手记', 'journal', x, y, 100)
    if area == 'town':
        x, y, _, _ = bounds('TownActions')
        face(ref, 'T 投果', 'fruit', x, y, 102)
        face(ref, '空格 等候', 'wait', x + 102, y, 116)
    else:
        x, y, _, _ = bounds('DelveActions')
        for label, name, width in [('Q 护卫', 'brace', 92), ('F 盾击', 'skill', 100),
                                   ('R 药水', 'drink', 92), ('空格 等候', 'wait', 116)]:
            face(ref, label, name, x, y, width)
            x += width
    x, y, _, _ = bounds('Mini')
    text(ref, x + 8, y + 5, '青：你  金：出入口')
    grid = (ROOT / f'assets/maps/{area}.grid').read_text().splitlines()
    for row, line in enumerate(grid):
        for col, glyph in enumerate(line):
            seen = (col - 10) ** 2 + (row - 11) ** 2 <= 20
            color = '#7f9187' if seen and glyph != '#' else '#121c1c'
            if seen and glyph == '#':
                color = '#3d3b33'
            if seen and glyph in '<>':
                color = '#f5c75e'
            if col == 10 and row == 11:
                color = '#7affdb'
            ImageDraw.Draw(ref).rectangle((x + 16 + col * 6, y + 28 + row * 6,
                                           x + 20 + col * 6, y + 32 + row * 6), fill=color)
    text(ref, x + 8, y + 121, '走过的地方留在图上')
    ref.resize((2560, 1440), Image.Resampling.NEAREST).save(ART / f'adventure-{area}-reference.png')

literals = [s[1:-1].replace('\\n', '\n') for s in re.findall(r'"(?:[^"\\]|\\.)*"', SOURCE)]


def copy(prefix, *values):
    value = next(s for s in literals if s.startswith(prefix))
    for replacement in values:
        value = value.replace('{}', str(replacement), 1)
    return value


pages = Image.new('RGBA', (1520, 1440), '#526155')
paper = Image.open(OUT / 'folio.png').convert('RGBA').resize((760, 700), Image.Resampling.NEAREST)


def page(x, y, title, summary, body, confirm):
    pages.alpha_composite(paper, (x, y))
    text(pages, x + 64, y + 32, title, heading, INK)
    for offset, value, color in [(92, summary, '#6d543b'), (206, body, INK)]:
        lines = value.split('\n')
        assert len(lines) * reading.size * 1.3 <= (84 if offset == 92 else 404), value
        for line, content in enumerate(lines):
            assert reading.getlength(content) <= 632, (content, reading.getlength(content))
            text(pages, x + 64, y + offset + round(line * reading.size * 1.3), content, reading, color)
    face(pages, confirm, None, x + 448, y + 630, 248, 44, reading)


page(0, 40, '工具箱 · 选一套器物', copy('旧箱子里躺着一面圆盾，', '圆盾'), '', 'E 带上圆盾')
for y, prefix, name in [(256, '圆盾 · 守住窄路', 'buckler'), (380, '钩索 · 隔格先手', 'hook')]:
    nine(pages, button, 64, y, 632, 108)
    pages.alpha_composite(Image.open(OUT / f'{name}-plate.png').resize((76, 76), Image.Resampling.NEAREST), (80, y + 16))
    text(pages, 172, y + 16, copy(prefix), reading)
page(760, 40, '灯火接你回家', copy('回程的灯提到了你身边。'),
     copy('这是第{}次被抬回旅店。', 2, 10, 10, 8), 'E 回到铜壶镇')
page(0, 740, '第一股泉水', copy('起初只有几滴水，'),
     copy('池底的灰先浮了起来。', 3, 4, 8, 1, '钩索'), 'E 继续旅行')
page(760, 740, '旅行手记 · 器物与操作',
     '装备：钩索   药水：2瓶\n器物技：已蓄势\n普攻、移动、护卫和等候都走一拍。',
     copy('WASD / 方向键组合；', '钩击：正面两格2伤，拉近'), '→ 下一页')
text(pages, 18, 8, 'TYPESETTING SPECIMENS / NOT GAME CAPTURES')
pages.save(ART / 'adventure-pages-reference.png')

atlas = Image.new('RGBA', (len(NAMES) * 48, 72), '#493a2b')
for i, name in enumerate(NAMES):
    atlas.alpha_composite(icons[name].resize((48, 48), Image.Resampling.NEAREST), (i * 48, 0))
    text(atlas, i * 48 + 1, 52, name, ImageFont.load_default())
atlas.save(ART / 'command-icons-reference.png')
layout = {'source': 'src/book.rs', 'logical_reference': [1600, 900],
          'physical_reference': [2560, 1440], 'font_defaults': sizes,
          'physical_font_px': {k: v * 1.6 for k, v in sizes.items()},
          'parts': {name: bounds(name) for name in
                    ('Place', 'Compass', 'Vitals', 'Notice', 'Guide', 'LeftActions',
                     'TownActions', 'DelveActions', 'Journal', 'Mini', 'Field')},
          'icons': {'size': [24, 24], 'names': NAMES, 'authorship': 'Original hand-authored pixel coordinates in make_adventure_book.py'},
          'modal': {'page': [760, 700], 'body': [64, 206, 632, 404], 'body_line_height': sizes['reading'] * 1.3}}
(ART / 'ui-layout.json').write_text(json.dumps(layout, ensure_ascii=False, indent=2) + '\n')
print('Exported original compact wood/brass stationery, nine command icons, north compass and 2560x1440 typography specimens.')
