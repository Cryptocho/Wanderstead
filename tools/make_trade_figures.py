#!/usr/bin/env python3
"""Export original crockery / apothecary cycles, retaining fixed boot pivots.

Run: python tools/make_trade_figures.py (Pillow >= 10).
Sources and generation provenance: art/figures/RIGHTS.txt.
This is an asset authoring step, not a game screenshot or runtime check.
"""
from collections import deque
import json
import math
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / 'art/figures'
OUT = ROOT / 'assets/figures'
TILE = (160, 128)
BODY = 108

# Timings match the explicit held-key poses in src/figures.rs.
CYCLES = {
    'keeper-crockery': {
        'phases': ['take plate', 'raise cloth', 'wipe left', 'wipe right',
                   'inspect plate', 'tilt to light', 'lower plate', 'offer clean plate'],
        'timeline': [[0, 1.10], [1, .26], [2, .24], [3, .24], [2, .24],
                     [3, .24], [2, .24], [3, .24], [4, .36], [5, 1.45],
                     [6, .32], [7, 1.35], [6, .32]],
        'world_foot': [-5, 0, 3],
    },
    'apothecary-supplies': {
        'phases': ['mortar ready', 'lift pestle', 'press herbs', 'grind herbs',
                   'take vial', 'raise vial', 'hold vial to light', 'lower vial'],
        'timeline': [[0, 1.20], [1, .28], [2, .24], [3, .34], [1, .28],
                     [2, .24], [3, .34], [0, .42], [4, .42], [5, .32],
                     [6, 1.90], [5, .32], [7, .42], [0, .65]],
        'world_foot': [6, 0, 2],
    },
    'seated-worker-cat': {
        'phases': ['rest by crate', 'offer hand', 'sniff palm', 'rub cheek',
                   'scratch ear', 'withdraw hand', 'sit at boot', 'curl up'],
        'timeline': [[0, 2.60], [1, .60], [2, .65], [3, 1.15], [4, 1.50],
                     [5, .50], [6, 1.25], [7, 3.00], [6, .60]],
        'world_foot': [7.55, 0, -5.85],
    },
}


def clean(cell, secondary=False):
    """Discard source matte/fringe and disconnected pixels, preserving internal greens."""
    cell = cell.convert('RGBA')
    alpha = cell.getchannel('A').point(lambda a: 255 if a >= 225 else 0)
    boundary = ImageChops.subtract(alpha, alpha.filter(ImageFilter.MinFilter(3)))
    for y in range(cell.height):
        for x in range(cell.width):
            if boundary.getpixel((x, y)):
                r, g, b, _ = cell.getpixel((x, y))
                if (r > 90 and r > g * 2.4 and r > b * 1.7) or (g > 65 and g > r * 1.5 and g > b * 1.7):
                    alpha.putpixel((x, y), 0)
    live = {(x, y) for y in range(cell.height) for x in range(cell.width)
            if alpha.getpixel((x, y))}
    components = []
    while live:
        start = live.pop()
        component = {start}
        queue = deque([start])
        while queue:
            x, y = queue.popleft()
            for p in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if p in live:
                    live.remove(p)
                    component.add(p)
                    queue.append(p)
        components.append(component)
    body = max(components, key=len)
    alpha = Image.new('L', cell.size)
    for component in components:
        if component is body or (secondary and len(component) >= len(body) * .02):
            for p in component:
                alpha.putpixel(p, 255)
    cell.putalpha(alpha)
    return cell


def export(name, details):
    source = Image.open(ART / 'source' / (name + '.webp'))
    sw, sh = source.width // 4, source.height // 2
    cells = [clean(source.crop((i % 4 * sw, i // 4 * sh,
                               (i % 4 + 1) * sw, (i // 4 + 1) * sh)),
                   secondary=name == 'seated-worker-cat') for i in range(8)]
    scale = BODY / max(im.getbbox()[3] - im.getbbox()[1] for im in cells)
    atlas = Image.new('RGBA', (TILE[0] * 4, TILE[1] * 2))
    pivots = []
    for i, im in enumerate(cells):
        box = im.getbbox()
        # Anchor to the boot pair, not to changing arms/plate/vial extents.
        # A separate cat must not pull the seated worker's boot/crate pivot.
        boot_left = sw // 3 if name == 'seated-worker-cat' else 0
        boot_box = im.crop((boot_left, box[3] - 30, sw, box[3])).getbbox()
        sole_x = boot_left + (boot_box[0] + boot_box[2]) * .5
        pivots.append([sole_x, box[3]])
        sprite = im.crop(box)
        sprite = sprite.resize((round(sprite.width * scale), round(sprite.height * scale)), Image.Resampling.LANCZOS)
        sprite.putalpha(sprite.getchannel('A').point(lambda a: 255 if a >= 150 else 0))
        x = i % 4 * TILE[0] + TILE[0] // 2 + round((box[0] - sole_x) * scale)
        y = i // 4 * TILE[1] + TILE[1] - 2 - sprite.height
        atlas.alpha_composite(sprite, (x, y))
    # One stable palette per complete cycle; no temporal quantisation flicker.
    alpha = atlas.getchannel('A')
    atlas = atlas.convert('RGB').quantize(80, dither=Image.Dither.NONE).convert('RGBA')
    atlas.putalpha(alpha)
    output_name = 'keeper-work' if name == 'keeper-crockery' else name
    atlas.save(OUT / (output_name + '.png'), optimize=True)
    details.update(asset='figures/' + output_name + '.png', tile=list(TILE), grid=[4, 2],
                   foot=[80, 126], body_pixels=BODY, palette_colors=80, alpha='binary',
                   source_sole_pivots=pivots, period=round(sum(p[1] for p in details['timeline']), 2))
    return atlas


atlases = {name: export(name, details) for name, details in CYCLES.items()}
# Contact is two small shoe-shaped pools, not a generic broad disc. Real upright
# alpha casters supply the directional sun shadow; this only grounds each sole.
contact = Image.new('RGBA', (128, 64))
for y in range(64):
    for x in range(128):
        amount = 0.
        for center in (39, 87):
            radius = math.sqrt(((x - center) / 21) ** 2 + ((y - 32) / 19) ** 2)
            amount = max(amount, max(0., 1. - radius) ** 1.6)
        contact.putpixel((x, y), (25, 24, 31, round(78 * amount)))
contact.save(OUT / 'sole-contact.png', optimize=True)

# Wider seated footprint: crate, two boots, and the cat's compact paw contact.
seated_contact = Image.new('RGBA', (192, 64))
for y in range(64):
    for x in range(192):
        amount = 0.
        for cx, cy, rx, ry, strength in [(64, 33, 17, 17, 70), (128, 33, 17, 17, 70),
                                        (96, 27, 26, 14, 42), (19, 35, 17, 13, 55)]:
            radius = math.sqrt(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2)
            amount = max(amount, strength * max(0., 1. - radius) ** 1.5)
        seated_contact.putpixel((x, y), (25, 24, 31, round(amount)))
seated_contact.save(OUT / 'seated-contact.png', optimize=True)

sheet = Image.new('RGB', (1296, 180 * len(atlases)), (198, 192, 178))
silhouettes = Image.new('RGB', sheet.size, (219, 213, 195))
draw, sil = ImageDraw.Draw(sheet), ImageDraw.Draw(silhouettes)
for row, (name, atlas) in enumerate(atlases.items()):
    for d in (draw, sil):
        d.text((8, 6 + row * 180), name + ' — fixed soles; held trade poses', fill=(48, 43, 46))
    for frame, phase in enumerate(CYCLES[name]['phases']):
        x, y = 8 + frame % 8 * TILE[0], 46 + row * 180
        sprite = atlas.crop((frame % 4 * TILE[0], frame // 4 * TILE[1],
                             (frame % 4 + 1) * TILE[0], (frame // 4 + 1) * TILE[1]))
        for d in (draw, sil):
            d.text((x, y - 23), phase, fill=(48, 43, 46))
        sheet.paste(sprite, (x, y), sprite)
        black = Image.new('RGBA', TILE, (32, 29, 37))
        black.putalpha(sprite.getchannel('A'))
        silhouettes.paste(black, (x, y), black)
        for d in (draw, sil):
            d.line((x + 76, y + 126, x + 84, y + 126), fill=(176, 67, 47))
sheet.save(ART / 'trade-contact-sheet.png', optimize=True)
silhouettes.save(ART / 'trade-silhouettes.png', optimize=True)
(ART / 'trade-atlas-layout.json').write_text(json.dumps(CYCLES, indent=2) + '\n')
print('Exported original trade and seated-cat cycles, source-art inspections and local contacts.')
