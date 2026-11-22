#!/usr/bin/env python3
"""Rebuild original generated character atlases. Requires Pillow >= 10.
Run from repository root: python tools/make_figures.py
Inputs and rights/provenance: art/figures/source/, art/figures/RIGHTS.txt.
No network access; all exported art has binary transparency and shared palette.
"""
from collections import deque
from pathlib import Path
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "art/figures/source"
OUT = ROOT / "assets/figures"
ART = ROOT / "art/figures"
OUT.mkdir(parents=True, exist_ok=True)


def clean(source):
    """Discard generated translucent matte and saturated edge artifacts.

    The model's nominally transparent walk source has a white alpha-143 matte.
    Alpha cutoff 210 separates that matte from real linen (alpha 240..254).
    Keep only connected body pixels so stray one-pixel noise cannot shift pivots.
    """
    im = source.convert("RGBA")
    pixels = im.load()
    w, h = im.size
    live = set()
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            edge = (r > 170 and g < 65 and b < 90) or (g > 205 and r < 100 and b < 125)
            if a >= 210 and not edge:
                live.add((x, y))
    components = []
    while live:
        start = live.pop()
        component = {start}
        queue = deque([start])
        while queue:
            x, y = queue.popleft()
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                point = x + dx, y + dy
                if point in live:
                    live.remove(point)
                    component.add(point)
                    queue.append(point)
        components.append(component)
    if not components:
        raise ValueError("Empty sprite cell")
    biggest = max(map(len, components))
    result = Image.new("RGBA", im.size)
    dst = result.load()
    for component in components:
        if len(component) >= max(16, biggest // 100):
            for x, y in component:
                r, g, b, _ = pixels[x, y]
                dst[x, y] = r, g, b, 255
    return result


def cells(file, cols, rows):
    im = Image.open(SOURCE / file)
    w, h = im.size
    return [clean(im.crop((x*w//cols, y*h//rows, (x+1)*w//cols, (y+1)*h//rows)))
            for y in range(rows) for x in range(cols)]


walk = cells("traveler-walk.webp", 4, 4)
actions = cells("traveler-actions.webp", 4, 3)
breakfast = cells("traveler-breakfast.webp", 4, 1)
keeper = cells("keeper-work.webp", 4, 2)
jelly_plum = cells("jelly-plum.webp", 4, 3)
# A common RGB palette for every direction/frame prevents hue flickering.
palette_source = Image.new("RGB", (1536, 1024), (30, 35, 38))
for i, sprite in enumerate(walk + actions + breakfast + keeper + jelly_plum):
    thumbnail = sprite.copy()
    thumbnail.thumbnail((128, 128))
    palette_source.paste(thumbnail, ((i % 12)*128, (i // 12)*128), thumbnail)
palette = palette_source.quantize(colors=64, method=Image.Quantize.MEDIANCUT)

# Madder-dyed cloak: preserve the source's hand-shaped value clusters. Replace
# only the cloth ramp in traveler frames, not the earth, the innkeeper or jelly.
cloth_dye = {
    (32,53,52):(62,33,43), (45,71,69):(93,41,46), (62,83,83):(119,51,49),
    (69,97,89):(148,61,49), (84,122,103):(180,77,54),
    (106,141,119):(199,99,64), (112,153,125):(215,117,77),
    (119,164,139):(232,140,94),
}
dyed_palette = [v for c in zip(*[iter(palette.getpalette())]*3) for v in cloth_dye.get(c, c)]


def export(name, source, cols, rows, tile, body_pixels, scale_reference=None):
    width, height = tile
    # Crouches and hop squashes retain their actual height. Scale is shared
    # over the whole animation; only boot/base baseline and horizontal center align.
    reference = scale_reference or max(s.getbbox()[3]-s.getbbox()[1] for s in source)
    scale = body_pixels / reference
    atlas = Image.new("RGBA", (cols*width, rows*height))
    for i, sprite in enumerate(source):
        bounds = sprite.getbbox()
        sprite = sprite.crop(bounds)
        size = max(1, round(sprite.width*scale)), max(1, round(sprite.height*scale))
        sprite = sprite.resize(size, Image.Resampling.LANCZOS)
        alpha = sprite.getchannel("A").point(lambda a: 255 if a >= 150 else 0)
        indexed = sprite.convert("RGB").quantize(palette=palette, dither=Image.Dither.NONE)
        if name is None or name == "traveler-walk":
            indexed.putpalette(dyed_palette)
        rgb = indexed.convert("RGB")
        sprite = rgb.convert("RGBA")
        sprite.putalpha(alpha)
        if sprite.width > width-4 or sprite.height > height-4:
            raise ValueError(f"{name} frame {i} exceeds its cell: {sprite.size}")
        # Two transparent rows below soles, shared by every action.
        x = (width-sprite.width)//2 + (i % cols)*width
        y = height-2-sprite.height + (i // cols)*height
        atlas.alpha_composite(sprite, (x, y))
    if name is not None:
        atlas.save(OUT / f"{name}.png", optimize=True)
    return atlas


atlases = {}
atlases["traveler-walk"] = export("traveler-walk", walk, 4, 4, (96, 128), 108)
action_strip = export(None, actions, 4, 3, (96, 128), 108)
breakfast_strip = export(None, breakfast, 4, 1, (96, 128), 108)
atlases["traveler-actions"] = Image.new("RGBA", (384, 512))
atlases["traveler-actions"].paste(action_strip, (0, 0))
atlases["traveler-actions"].paste(breakfast_strip, (0, 384))
atlases["traveler-actions"].save(OUT / "traveler-actions.png", optimize=True)
atlases["keeper-work"] = export("keeper-work", keeper, 4, 2, (96, 128), 108)
atlases["jelly"] = export("jelly", jelly_plum[:8], 4, 2, (96, 96), 78)
atlases["plum"] = export("plum", jelly_plum[8:9], 1, 1, (32, 32), 25)

# Small grounding shadow, separate from the sun-cast upright silhouette.
contact_shadow = Image.new("RGBA", (64, 64))
for y in range(64):
    for x in range(64):
        radius = (((x-31.5)**2+(y-31.5)**2)**0.5)/31.5
        alpha = round(100*max(0.0, 1.0-radius)**1.4)
        contact_shadow.putpixel((x,y), (15,20,21,alpha))
contact_shadow.save(OUT / "contact-shadow.png", optimize=True)

# Contact sheet is an art inspection artifact, not a claimed in-engine screenshot.
contact = Image.new("RGB", (816, 930), (49, 57, 62))
draw = ImageDraw.Draw(contact)
placements = [("traveler-walk", 8, 24), ("traveler-actions", 416, 24),
              ("keeper-work", 416, 568), ("jelly", 8, 580), ("plum", 416, 852)]
for name, x, y in placements:
    draw.text((x, y-16), name, fill=(244, 231, 207))
    atlas = atlases[name]
    contact.paste(atlas, (x, y), atlas)
contact.save(ART / "contact-sheet.png")

sample = Image.new("RGB", (480, 160), (49, 57, 62))
for column, index in enumerate((0, 4, 8, 12)):
    sample.paste(atlases["traveler-walk"].crop((0, (index//4)*128, 96, (index//4+1)*128)),
                 (column*120+12, 16), atlases["traveler-walk"].crop((0, (index//4)*128, 96, (index//4+1)*128)))
sample.resize((960, 320), Image.Resampling.NEAREST).save(ART / "traveler-sample.png")
metadata = {
    "palette_colors": 64, "sampler": "nearest", "alpha": "binary", "padding_below_feet": 2,
    "traveler-walk": {"tile": [96,128], "grid": [4,4], "rows": ["south","west","north","east"], "foot": [48,126], "body_pixels":108},
    "traveler-actions": {"tile":[96,128], "grid":[4,4], "rows":["gather","shake","throw","eat"], "foot":[48,126], "body_pixels":108},
    "keeper-work": {"tile":[96,128], "grid":[4,2], "rows":["stir","knead"], "foot":[48,126], "body_pixels":108},
    "jelly": {"tile":[96,96], "grid":[4,2], "rows":["hop","swallow"], "foot":[48,94], "body_pixels":78},
    "plum": {"tile":[32,32], "grid":[1,1], "foot":[16,30], "body_pixels":25},
}
(ART / "atlas-layout.json").write_text(json.dumps(metadata, indent=2)+"\n")
print("Exported five offline atlases, common 64-color palette, binary alpha; art/figures/contact-sheet.png")
