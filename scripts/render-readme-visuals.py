#!/usr/bin/env python3
"""Renders the README pictures from the interface prototype (docs/prototype/index.html).

Usage: scripts/render-readme-visuals.py   (needs python3 + playwright with Chromium)
Writes docs/hero.png and docs/steps.png.
"""
import pathlib
import sys
from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parent.parent
PAGE = (ROOT / "docs/prototype/index.html").as_uri()
errors = []


def hide_toast(page):
    page.evaluate("document.querySelector('#toast').classList.remove('on')")
    page.wait_for_timeout(300)


def center(page, dx=0, dy=0):
    box = page.locator("#c").bounding_box()
    return box["x"] + box["width"] / 2 + dx, box["y"] + box["height"] / 2 + dy


def click_ground(page, wx, wy):
    """Clicks the screen point that sits over the ground point (wx, wy)."""
    px, py = page.evaluate(
        """([x, y]) => { const P = window.SolidSketchPrototype; const c = document.querySelector('#c').getBoundingClientRect();
            // forward projection copied from proj(), at z = 0
            const S = P.S, cos = Math.cos(S.yaw), sin = Math.sin(S.yaw), rx = x*cos - y*sin, ry = x*sin + y*cos, k = S.scale;
            const ox = c.width/2 + S.panX, oy = c.height*.56 + S.panY;
            return [c.left + ox + (rx-ry)*Math.cos(Math.PI/6)*k, c.top + oy + (rx+ry)*.5*k]; }""",
        [wx, wy],
    )
    page.mouse.move(px, py)
    page.mouse.down()
    page.mouse.up()
    return px, py


with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={"width": 1280, "height": 780}, device_scale_factor=2)
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.goto(PAGE)
    page.wait_for_timeout(200)

    # Hero: the finished scene with the sketchy edge style on.
    page.click("#style")
    page.click("#tools button[data-id=select]")
    page.mouse.click(544, 397)
    page.wait_for_timeout(100)
    page.screenshot(path=str(ROOT / "docs/hero.png"))
    page.click("#style")  # back to clean edges for the step pictures

    # Steps: draw a rectangle, pull it up, paint it.
    shots = []
    clip = {"x": 68, "y": 48, "width": 964, "height": 698}  # the canvas area only
    page.evaluate("window.SolidSketchPrototype.S.objs = []; window.SolidSketchPrototype.draw()")
    page.click("#swatches button:nth-child(1)")  # Plaster, so the paint step shows
    page.click("#tools button[data-id=rect]")
    click_ground(page, -2, -1.5)
    page.mouse.move(*center(page, 0, 0))
    px, py = page.evaluate(
        """() => { const P = window.SolidSketchPrototype, S = P.S, c = document.querySelector('#c').getBoundingClientRect();
            const cos = Math.cos(S.yaw), sin = Math.sin(S.yaw), x = 2.5, y = 1.5, rx = x*cos - y*sin, ry = x*sin + y*cos, k = S.scale;
            return [c.left + c.width/2 + S.panX + (rx-ry)*Math.cos(Math.PI/6)*k, c.top + c.height*.56 + S.panY + (rx+ry)*.5*k]; }"""
    )
    page.mouse.move(px, py)
    page.keyboard.type("5,3")
    page.wait_for_timeout(80)
    page.screenshot(path=str(ROOT / "docs/_step1.png"), clip=clip)
    page.keyboard.press("Enter")

    page.click("#tools button[data-id=pushpull]")
    # find the top face and drag it up
    face = page.evaluate(
        """() => { const P = window.SolidSketchPrototype, S = P.S, o = S.objs[0], c = document.querySelector('#c').getBoundingClientRect();
            const cos = Math.cos(S.yaw), sin = Math.sin(S.yaw), x = o.x + o.w/2, y = o.y + o.d/2, rx = x*cos - y*sin, ry = x*sin + y*cos, k = S.scale;
            return [c.left + c.width/2 + S.panX + (rx-ry)*Math.cos(Math.PI/6)*k, c.top + c.height*.56 + S.panY + (rx+ry)*.5*k - o.h*k]; }"""
    )
    page.mouse.move(*face)
    page.mouse.down()
    page.mouse.move(face[0], face[1] - 120, steps=8)
    page.wait_for_timeout(80)
    hide_toast(page)
    page.screenshot(path=str(ROOT / "docs/_step2.png"), clip=clip)
    page.mouse.up()

    page.click("#swatches button:nth-child(2)")  # Brick, selects Paint
    for key in ("left", "right"):
        hit = page.evaluate(
            """(key) => { const P = window.SolidSketchPrototype; P.draw();
                return null; }""",
            key,
        )
    # click a left-hand side face of the block
    side = page.evaluate(
        """() => { const P = window.SolidSketchPrototype, S = P.S, o = S.objs[0], c = document.querySelector('#c').getBoundingClientRect();
            const cos = Math.cos(S.yaw), sin = Math.sin(S.yaw), x = o.x + o.w/2, y = o.y + o.d, rx = x*cos - y*sin, ry = x*sin + y*cos, k = S.scale;
            return [c.left + c.width/2 + S.panX + (rx-ry)*Math.cos(Math.PI/6)*k, c.top + c.height*.56 + S.panY + (rx+ry)*.5*k - o.h*k/2]; }"""
    )
    page.mouse.click(*side)
    page.click("#tools button[data-id=select]")
    page.wait_for_timeout(80)
    hide_toast(page)
    page.screenshot(path=str(ROOT / "docs/_step3.png"), clip=clip)
    browser.close()

if errors:
    print("page errors:", *errors, sep="\n  ")
    sys.exit(1)
print("ok")

# Join the three step pictures into one strip with a caption under each.
from PIL import Image, ImageDraw, ImageFont

steps = [("1  Draw", "_step1.png"), ("2  Pull", "_step2.png"), ("3  Paint", "_step3.png")]
imgs = [Image.open(ROOT / "docs" / f).convert("RGB") for _, f in steps]
w, h = 900, int(imgs[0].height * 900 / imgs[0].width)
imgs = [im.resize((w, h), Image.LANCZOS) for im in imgs]
pad, cap = 18, 64
strip = Image.new("RGB", (3 * w + 4 * pad, h + cap + 2 * pad), "#f4f0e6")
draw = ImageDraw.Draw(strip)
try:
    font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 34)
except OSError:
    font = ImageFont.load_default()
for i, (im, (label, _)) in enumerate(zip(imgs, steps)):
    x = pad + i * (w + pad)
    strip.paste(im, (x, pad))
    draw.text((x + 8, pad + h + 12), label, fill="#27334d", font=font)
strip.save(ROOT / "docs/steps.png", optimize=True)
for _, f in steps:
    (ROOT / "docs" / f).unlink()
print("steps.png written")
