#!/usr/bin/env python3
"""Export original short-proportioned JRPG art, without anatomical rescaling.

Run from repository root: python tools/make_figures.py
Sources/provenance: art/figures/source and art/figures/RIGHTS.txt.
Exports include a black silhouette inspection sheet and stable sole pivots.
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
    """Remove model matte and chroma key, retaining actual connected artwork."""
    im = source.convert("RGBA")
    live = set()
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, a = im.getpixel((x, y))
            magenta = r > 145 and b > 125 and g < 115 and min(r, b) > g * 1.5
            if a >= 210 and not magenta:
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
    for component in components:
        if len(component) >= max(12, biggest // 150):
            for point in component:
                r, g, b, _ = im.getpixel(point)
                result.putpixel(point, (r, g, b, 255))
    return result


def cells(file, cols, rows):
    im = Image.open(SOURCE / file)
    w, h = im.size
    return [clean(im.crop((x*w//cols, y*h//rows, (x+1)*w//cols, (y+1)*h//rows)))
            for y in range(rows) for x in range(cols)]


sources = {
    "traveler-walk": cells("traveler-short-walk.png", 4, 4),
    "traveler-diagonal": cells("traveler-diagonal.png", 4, 4),
    "traveler-actions": cells("traveler-short-actions.png", 4, 4),
    "traveler-combat": cells("traveler-combat.png", 4, 4),
    "copper-sentry": cells("copper-sentry.png", 4, 4),
    "phosphor-moth": cells("phosphor-moth.png", 4, 4),
}
jelly_plum = cells("jelly-plum.webp", 4, 3)
sources["jelly"] = jelly_plum[:8]
sources["plum"] = jelly_plum[8:9]

# Quantize once, with all original material ramps, to prevent animation flicker.
palette_source = Image.new("RGB", (1536, 1280), (32, 36, 43))
for i, sprite in enumerate(s for group in sources.values() for s in group):
    thumb = sprite.copy()
    thumb.thumbnail((128, 128))
    palette_source.paste(thumb, ((i % 12)*128, (i // 12)*128), thumb)
palette = palette_source.quantize(colors=96, method=Image.Quantize.MEDIANCUT)

metadata = {
    "palette_colors": 96, "sampler": "nearest", "alpha": "binary",
    "padding_below_feet": 2, "anatomy": "original short 2.5-3-head figures; uniform scale only",
    "trade_cycles": "trade-atlas-layout.json",
    "foreground_visibility": "whole-object ownership; no traveler aperture masks",
}
atlases = {}


def export(name, cols, rows, tile, body_pixels, row_names, reference_rows=None):
    frames = sources[name]
    width, height = tile
    reference = frames[:cols*reference_rows] if reference_rows else frames
    body = max(s.getbbox()[3]-s.getbbox()[1] for s in reference)
    # ONE uniform scale for the complete animation, including collapse frames.
    extent = max(max(s.width/2-s.getbbox()[0], s.getbbox()[2]-s.width/2)*2 for s in frames)
    scale = min(body_pixels / body, (width-8) / extent)
    atlas = Image.new("RGBA", (cols*width, rows*height))
    for i, source in enumerate(frames):
        bounds = source.getbbox()
        sprite = source.crop(bounds)
        sprite = sprite.resize((max(1, round(sprite.width*scale)),
                                max(1, round(sprite.height*scale))), Image.Resampling.LANCZOS)
        alpha = sprite.getchannel("A").point(lambda a: 255 if a >= 145 else 0)
        sprite = sprite.convert("RGB").quantize(palette=palette, dither=Image.Dither.NONE).convert("RGBA")
        sprite.putalpha(alpha)
        # Preserve the source's body/boot center, not the changing weapon/wing bounds.
        x = width//2 + round((bounds[0]-source.width/2)*scale) + (i % cols)*width
        y = height-2-sprite.height + (i // cols)*height
        atlas.alpha_composite(sprite, (x, y))
    atlas.save(OUT / f"{name}.png", optimize=True)
    atlases[name] = atlas
    metadata[name] = {"tile": list(tile), "grid": [cols, rows], "rows": row_names,
                      "foot": [width//2, height-2], "body_pixels": body_pixels}


export("traveler-walk", 4, 4, (160,128), 108, ["south","west","north","east"])
export("traveler-diagonal",4,4,(160,128),108,["southwest","northwest","northeast","southeast"])
export("traveler-actions", 4, 4, (160,128), 108, ["gather","shake","throw","eat"])
export("traveler-combat", 4, 4, (160,128), 108, ["strike","brace","hurt/down","drink"], 2)
export("copper-sentry", 4, 4, (128,128), 108, ["idle","prepare","strike","down"], 1)
export("phosphor-moth", 4, 4, (128,128), 94, ["idle","prepare","dive","down"], 1)
export("jelly", 4, 2, (96,96), 78, ["hop","swallow"])
export("plum", 1, 1, (32,32), 25, ["plum"])

# Portrait uses the newly drawn large head, never the obsolete long figure.
front = sources["traveler-walk"][1]
box = front.getbbox()
portrait = front.crop((box[0], box[1], box[2], box[1]+round((box[3]-box[1])*.60)))
portrait.thumbnail((104,104), Image.Resampling.LANCZOS)
portrait_card = Image.new("RGBA", (128,128))
portrait_card.alpha_composite(portrait, ((128-portrait.width)//2, (128-portrait.height)//2))
portrait_card.save(OUT / "traveler-portrait.png")

shadow = Image.new("RGBA", (64,64))
for y in range(64):
    for x in range(64):
        radius = (((x-31.5)**2+(y-31.5)**2)**.5)/31.5
        shadow.putpixel((x,y), (15,20,21,round(105*max(0.,1.-radius)**1.4)))
shadow.save(OUT / "contact-shadow.png", optimize=True)

# Every row is shown at native pixels, plus black-on-linen silhouette evidence.
contact = Image.new("RGB", (1312, ((len(atlases)+1)//2)*552+8), (43,49,54))
silhouettes = Image.new("RGB", contact.size, (215,208,184))
draw = ImageDraw.Draw(contact)
sil = ImageDraw.Draw(silhouettes)
for i, name in enumerate(atlases):
    x = (i % 2)*656 + 8
    y = (i // 2)*552 + 24
    atlas = atlases[name]
    draw.text((x,y-18), name, fill=(234,219,190))
    sil.text((x,y-18), name, fill=(30,35,37))
    contact.paste(atlas, (x,y), atlas)
    black = Image.new("RGBA", atlas.size, (25,30,32,255))
    black.putalpha(atlas.getchannel("A"))
    silhouettes.paste(black, (x,y), black)
    tile = metadata[name]["tile"]
    cols, rows = metadata[name]["grid"]
    for row in range(rows):
        for col in range(cols):
            px, py = x+col*tile[0]+tile[0]//2, y+(row+1)*tile[1]-2
            draw.line((px-3,py,px+3,py), fill=(236,100,81))
            sil.line((px-3,py,px+3,py), fill=(167,66,47))
contact.save(ART / "contact-sheet.png")
silhouettes.save(ART / "silhouette-check.png")
sample = atlases["traveler-walk"].crop((0,0,640,128))
sample.resize((1280,256), Image.Resampling.NEAREST).save(ART / "traveler-sample.png")
(ART / "atlas-layout.json").write_text(json.dumps(metadata, indent=2)+"\n")
print("Exported walking and action atlases, portrait and silhouette study.")

# Trade workers use held object key poses and per-boot anchoring.
import make_trade_figures
