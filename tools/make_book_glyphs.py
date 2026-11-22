"""Check complete book + game copy against the bundled, unmodified OFL face.
Run: uv run --with fonttools python tools/make_book_glyphs.py
"""
from pathlib import Path
from fontTools.ttLib import TTFont
import hashlib, json, re
root=Path(__file__).resolve().parents[1]
font_path=root/'assets/book/FusionPixel.ttf'
font=TTFont(font_path)
chars=set()
sources=['src/book.rs','src/game.rs','src/delve.rs','src/controls.rs']
for source in sources:
 text=(root/source).read_text()
 for literal in re.findall(r'"(?:[^"\\]|\\.)*"',text):
  # Escapes in Rust UI strings are ASCII linebreaks/quotes; their backslash
  # spellings are supported ASCII too, so retaining them is conservative.
  chars.update(literal[1:-1])
chars.update(chr(n) for n in range(32,127))
chars.difference_update('\n\r\t')
missing=sorted(chars-set(map(chr,font.getBestCmap())))
report={'font':'Fusion Pixel 12px Proportional zh_hans 2026.11.10',
 'sha256':hashlib.sha256(font_path.read_bytes()).hexdigest(),
 'checked_characters':len(chars),'font_cmap_entries':len(font.getBestCmap()),
 'missing':missing,'checked_sources':sources}
book_source=(root/'src/book.rs').read_text()
sizes={key:float(re.search(rf'{key}:\s*([0-9.]+)',book_source).group(1))
       for key in ('hud','reading','heading')}
report['ui_font_defaults']=sizes
report['physical_font_px_at_1440p']={key:value*1.6 for key,value in sizes.items()}
(root/'art/book/glyph-coverage.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False))
if missing:raise SystemExit('Missing glyphs: '+''.join(missing))
