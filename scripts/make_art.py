#!/usr/bin/env python3
"""Turn an image into art XML for the stats card.

    scripts/make_art.py photo.jpg --cols 40 --rows 30 --out assets/art.xml

The picture itself is made by jp2a (truecolor ASCII); this script only prepares
the input and converts the output: it crops the image to the art box's aspect
ratio -- a cell is 0.6 em wide and 0.9 em tall, so the box is
(cols * 0.6) / (rows * 0.9) -- choosing the crop by where the image has the most
detail, then turns jp2a's ANSI stream into the <glyph fg="..."> markup the card
reads.
"""

import argparse
import subprocess
import tempfile
import xml.sax.saxutils as sax
from pathlib import Path

import numpy as np
from PIL import Image

# Cell metrics of the card: glyph advance 0.6 em, line height 1.5 x advance.
CELL_W = 0.6
CELL_H = 0.9


def crop_box(size, win_w, win_h, detail, bias):
    """Place a window of the given size where the image has the most detail.

    Windows slide independently per axis; `bias` nudges the choice towards the
    top, which is usually where faces are.
    """
    w, h = size
    win_w, win_h = min(win_w, w), min(win_h, h)

    def offset(profile, win):
        if win >= len(profile):
            return 0
        summed = np.convolve(profile, np.ones(win), mode="valid")
        return int(np.argmax(summed + bias * np.linspace(1, 0, len(summed)) * summed.max()))

    x0 = offset(detail.sum(axis=0), win_w)
    y0 = offset(detail.sum(axis=1), win_h)
    return (x0, y0, x0 + win_w, y0 + win_h)


def prepare(image_path, cols, rows, zoom, bias, lift):
    """Crop the image to the grid's aspect ratio and write it to a temp file."""
    img = Image.open(image_path).convert("RGB")
    gray = np.asarray(img.convert("L"), dtype=float) / 255.0
    gy, gx = np.gradient(gray)
    detail = np.hypot(gx, gy)

    aspect = (cols * CELL_W) / (rows * CELL_H)
    w, h = img.size
    base_w, base_h = (h * aspect, h) if w / h > aspect else (w, w / aspect)
    win_w, win_h = max(1, int(base_w / zoom)), max(1, int(base_h / zoom))
    box = crop_box(img.size, win_w, win_h, detail, bias)

    crop = img.crop(box)
    if lift != 1.0:
        crop = crop.point(lambda x: min(255, round((x / 255) ** lift * 255)))
    tmp = tempfile.NamedTemporaryFile(suffix=".png", delete=False)
    crop.save(tmp.name)
    return tmp.name, box


def jp2a(path, cols, rows, invert):
    """Render the crop with jp2a.

    `--invert` suits light-background artwork (jp2a then prints dark pixels as
    the dense end of its palette); dark artwork wants the plain mapping, so its
    background stays empty instead of turning into a wall of dense glyphs.
    """
    cmd = ["jp2a", "--colors", "--size=%dx%d" % (cols, rows)]
    if invert:
        cmd.append("--invert")
    cmd.append(path)
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def parse_ansi(text):
    """Pull (character, colour) cells out of jp2a's 24-bit colour output."""
    rows, row = [], []
    fg = (255, 255, 255)
    i = 0
    while i < len(text):
        ch = text[i]
        if ch == "\x1b":
            end = text.index("m", i)
            code = text[i + 2 : end]
            if code.startswith("38;2;"):
                fg = tuple(int(part) for part in code.split(";")[2:5])
            elif code in ("0", "39"):
                fg = (255, 255, 255)
            i = end + 1
        elif ch == "\n":
            rows.append(row)
            row = []
            i += 1
        else:
            row.append((ch, fg))
            i += 1
    if row:
        rows.append(row)
    return rows


def to_xml(rows):
    lines = ['<?xml version="1.0" encoding="UTF-8"?>', "<art>"]
    for row in rows:
        lines.append("  <line>")
        for ch, fg in row:
            if ch == " ":
                continue  # blank cell: nothing to draw
            lines.append('    <glyph fg="#%02x%02x%02x">%s</glyph>' % (*fg, sax.escape(ch)))
        lines.append("  </line>")
    lines.append("</art>")
    return "\n".join(lines) + "\n"


def main():
    p = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    p.add_argument("image")
    p.add_argument("--cols", type=int, default=40, help="grid columns (default 40)")
    p.add_argument("--rows", type=int, default=30, help="grid rows (default 30)")
    p.add_argument("--out", help="output art XML (default art-<image name>.xml)")
    p.add_argument("--zoom", type=float, default=1.0, help=">1 crops closer to the subject")
    p.add_argument("--bias", type=float, default=0.0, help=">0 keeps the top of the image")
    p.add_argument("--lift", type=float, default=0.7, help="exposure curve; <1 brightens dark art")
    p.add_argument("--invert", action="store_true", help="jp2a --invert, for light-background art")
    args = p.parse_args()

    crop_path, box = prepare(args.image, args.cols, args.rows, args.zoom, args.bias, args.lift)
    rows = parse_ansi(jp2a(crop_path, args.cols, args.rows, args.invert))
    Path(crop_path).unlink()

    out = Path(args.out or f"art-{Path(args.image).stem}.xml")
    out.write_text(to_xml(rows))
    print(f"{out}  {max(len(r) for r in rows)}x{len(rows)}  crop {box}")


if __name__ == "__main__":
    main()
