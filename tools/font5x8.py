#!/usr/bin/env python3
"""Bake the X11 misc-fixed 5x8 bitmap font (public domain: "Public domain
font. Share and enjoy.") into data/font5x8.txt for the credits' dot text.

Format: one block per printable ASCII character: a line with the character
code, then 8 lines of 5 cells ('#' ink, '.' blank).
"""
import gzip
import io
import os
from PIL import PcfFontFile, Image

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = "/usr/share/fonts/X11/misc/5x8.pcf.gz"
OUT = os.path.join(HERE, "../data/font5x8.txt")

pcf = PcfFontFile.PcfFontFile(io.BytesIO(gzip.open(SRC).read()))
with open(OUT, "w") as out:
    out.write("# X11 misc-fixed 5x8, public domain (tools/font5x8.py)\n")
    for code in range(32, 127):
        glyph = pcf.glyph[code]
        # each glyph image is already the full 5x8 cell
        img = glyph[3] if glyph else Image.new("1", (5, 8), 0)
        out.write(f"{code}\n")
        for y in range(8):
            out.write("".join("#" if img.getpixel((x, y)) else "." for x in range(5)) + "\n")
print("wrote", os.path.normpath(OUT))
