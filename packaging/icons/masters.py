#!/usr/bin/env python3
# Writes the icon masters, drawn on Papirus's 192 grid.
import os
import sys

out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), '../../assets/icons')


def tint(hex_, amount):
    # Mixes a colour towards white.
    rgb = [int(hex_[i:i + 2], 16) for i in (1, 3, 5)]
    return '#%02X%02X%02X' % tuple(round(c + (255 - c) * amount) for c in rgb)


def rect(x, y, w, h, r):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}"/>'


def circle(x, y, r):
    return f'<circle cx="{x}" cy="{y}" r="{r}"/>'


def path(d):
    return f'<path d="{d}"/>'


def line(points, width):
    pts = ' '.join(f'{x},{y}' for x, y in points)
    return (f'<polyline points="{pts}" fill="none" stroke-width="{width}" '
            'stroke-linecap="round" stroke-linejoin="round"/>')


def curve(d, width):
    return f'<path d="{d}" fill="none" stroke-width="{width}" stroke-linecap="round"/>'


# The page: 128 by 160, top-right corner folded.
PAGE = 'M40 16H112L160 64V168A8 8 0 0 1 152 176H40A8 8 0 0 1 32 168V24A8 8 0 0 1 40 16Z'
FOLD = 'M112 16V56A8 8 0 0 0 120 64H160Z'

# Part: (svg, colour, shadow, small). Shadow 'body', 'part' or None.
APPS = {}

BLUE = '#3D7FE5'
APPS['ely-doc'] = [
    (path(PAGE), BLUE, 'body', True), (path(FOLD), tint(BLUE, .45), 'fold', True),
    (rect(56, 84, 64, 12, 4), '#FFFFFF', 'part', True),
    (rect(56, 108, 80, 8, 4), '#FFFFFF', 'part', True),
    (rect(56, 124, 80, 8, 4), '#FFFFFF', 'part', True),
    (rect(56, 140, 80, 8, 4), '#FFFFFF', 'part', False),
    (rect(56, 156, 48, 8, 4), tint(BLUE, .55), 'part', False),
]

GREEN = '#2E9E5B'
APPS['ely-table'] = [
    (path(PAGE), GREEN, 'body', True), (path(FOLD), tint(GREEN, .45), 'fold', True),
    (path('M52 92H140V156H52ZM60 108H92V120H60ZM100 108H132V120H100Z'
          'M60 128H92V140H60ZM100 128H132V140H100Z').replace('<path', '<path fill-rule="evenodd"'),
     '#FFFFFF', 'part', True),
    (rect(100, 128, 32, 12, 0), '#FFD54F', None, False),
]

ORANGE = '#F07B2A'
APPS['ely-show'] = [
    (line([(96, 136), (64, 176)], 12), '#5D5D5D', 'part', True),
    (line([(96, 136), (128, 176)], 12), '#5D5D5D', 'part', True),
    (rect(16, 28, 160, 112, 10), ORANGE, 'body', True),
    (rect(16, 28, 160, 16, 8), tint(ORANGE, .3), None, False),
    (rect(32, 52, 128, 72, 4), '#FFFFFF', 'part', True),
    (rect(48, 96, 16, 16, 2), ORANGE, None, True),
    (rect(72, 80, 16, 32, 2), ORANGE, None, True),
    (rect(96, 66, 16, 46, 2), ORANGE, None, True),
    (path('M136 66A16 16 0 1 0 152 82H136Z'), '#5D7FC9', None, False),
]

RED = '#E0344F'
APPS['ely-pdf'] = [
    (path(PAGE), RED, 'body', True), (path(FOLD), tint(RED, .45), 'fold', True),
    (rect(52, 88, 88, 60, 6), '#FFFFFF', 'part', True),
    (path('M60 140L82 112L98 128L108 118L132 140Z'), RED, None, True),
    (circle(112, 104, 7), tint(RED, .4), None, False),
    (rect(52, 156, 56, 8, 4), tint(RED, .55), 'part', False),
]

SKY = '#1E8FE0'
APPS['ely-mail'] = [
    (rect(16, 40, 160, 112, 12), SKY, 'body', True),
    (path('M28 40H164L104 92A12 12 0 0 1 88 92Z'), tint(SKY, .35), None, True),
    (line([(30, 54), (96, 108), (162, 54)], 12), '#FFFFFF', 'part', True),
    (line([(32, 138), (72, 104)], 8), tint(SKY, .35), None, False),
    (line([(160, 138), (120, 104)], 8), tint(SKY, .35), None, False),
]

PURPLE = '#7E57C2'
APPS['ely-tree'] = [
    (line([(64, 44), (64, 148)], 20), PURPLE, 'part', True),
    (curve('M64 128C64 92 128 104 128 72', 20), PURPLE, 'part', True),
    (circle(64, 40, 24), PURPLE, 'part', True), (circle(64, 40, 11), '#FFFFFF', None, True),
    (circle(128, 64, 24), PURPLE, 'part', True), (circle(128, 64, 11), '#FFFFFF', None, True),
    (circle(64, 152, 24), '#F2A900', 'part', True), (circle(64, 152, 11), '#FFFFFF', None, True),
]

APPS['ely-mark'] = [
    (rect(28, 24, 136, 152, 10), '#ECECEC', 'body', True),
    (path('M38 24H154A10 10 0 0 1 164 34V52H28V34A10 10 0 0 1 38 24Z'), '#4F4F4F', None, True),
    (rect(46, 32, 10, 14, 5), '#FFFFFF', None, False), (rect(70, 32, 10, 14, 5), '#FFFFFF', None, False),
    (rect(94, 32, 10, 14, 5), '#FFFFFF', None, False), (rect(118, 32, 10, 14, 5), '#FFFFFF', None, False),
    (rect(142, 32, 10, 14, 5), '#FFFFFF', None, False),
    (path('M44 156V76H66L82 100L98 76H120V156H100V110L82 136L64 110V156Z'), '#3E4A5C', 'part', True),
    (path('M128 76H148V124H164L138 158L112 124H128Z'), '#F2A900', 'part', True),
]

APPS['ely-agent'] = [
    (rect(16, 32, 160, 140, 12), '#4F4F4F', 'body', True),
    (rect(16, 32, 160, 24, 12), '#5F5F5F', None, False),
    (rect(16, 44, 160, 12, 0), '#5F5F5F', None, False),
    (circle(32, 44, 4), '#FF6B8B', None, False), (circle(46, 44, 4), '#FFC857', None, False),
    (circle(60, 44, 4), '#5FD38D', None, False),
    (line([(44, 92), (72, 116)], 16), '#FFFFFF', 'part', True),
    (line([(72, 116), (44, 140)], 16), '#A2A2A2', 'part', True),
    (rect(84, 132, 48, 16, 4), '#FFFFFF', 'part', True),
    (path('M148 64C151 84 156 89 176 92C156 95 151 100 148 120C145 100 140 95 120 92C140 89 145 84 148 64Z'),
     '#FF6B8B', 'part', True),
]

# Apple's squircle on the 256 canvas; n = 5.
SQ = ('M231 128C231 229.764 229.764 231 128 231C26.236 231 25 229.764 25 128'
      'C25 26.236 26.236 25 128 25C229.764 25 231 26.236 231 128Z')
SHADOWS = {'body': (4, 2.88), 'fold': (2, 0.96), 'part': (2, 1.28)}


def paint(svg, colour):
    if 'stroke-width' in svg:
        return svg.replace('fill="none"', f'fill="none" stroke="{colour}"', 1)
    return svg.replace('/>', f' fill="{colour}"/>', 1)


def draw(parts, small):
    body, filters = [], set()
    for svg, colour, shadow, keep in parts:
        if small and not keep:
            continue
        if shadow and not small:
            dy, _ = SHADOWS[shadow]
            filters.add(shadow)
            body.append(f'<g transform="translate(0 {dy})" opacity="0.2" filter="url(#{shadow})">'
                        f'{paint(svg, "#000000")}</g>')
        body.append(paint(svg, colour))
    return ''.join(body), filters


def svg(parts, macos, small):
    body, used = draw(parts, small)
    defs = ''.join(f'<filter id="{name}" x="-0.2" y="-0.2" width="1.4" height="1.4">'
                   f'<feGaussianBlur stdDeviation="{SHADOWS[name][1]}"/></filter>'
                   for name in sorted(used))
    if macos:
        # A light squircle; the object fills 86 % of it.
        scale = 0.86 * 206 / 192
        offset = 128 - 96 * scale
        content = (f'<path fill="#F4F4F2" d="{SQ}"/>'
                   f'<g transform="translate({offset:.3f} {offset + 4:.3f}) scale({scale:.4f})">{body}</g>')
    else:
        content = f'<g transform="scale({256 / 192:.6f})">{body}</g>'
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">'
            f'<defs>{defs}</defs>{content}</svg>\n')


# macOS document: white page, the app's object on it.
SHEET = 'M60 16H164L212 64V232A8 8 0 0 1 204 240H60A8 8 0 0 1 52 232V24A8 8 0 0 1 60 16Z'
CORNER = 'M164 16V56A8 8 0 0 0 172 64H212Z'
# Apps that open files; ElyTree opens folders.
DOCUMENTS = ('ely-doc', 'ely-table', 'ely-show', 'ely-pdf', 'ely-mark', 'ely-mail')


def document(parts, small):
    body, used = draw(parts, small)
    used.add('body')
    defs = ''.join(f'<filter id="{name}" x="-0.2" y="-0.2" width="1.4" height="1.4">'
                   f'<feGaussianBlur stdDeviation="{SHADOWS[name][1]}"/></filter>'
                   for name in sorted(used))
    scale = 0.62
    offset_x, offset_y = 132 - 96 * scale, 140 - 96 * scale
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">'
            f'<defs>{defs}</defs>'
            f'<g transform="translate(0 3)" opacity="0.2" filter="url(#body)"><path d="{SHEET}"/></g>'
            f'<path fill="#FFFFFF" stroke="#D6D6D6" stroke-width="1.5" d="{SHEET}"/>'
            f'<path fill="#E4E4E4" d="{CORNER}"/>'
            f'<g transform="translate({offset_x:.3f} {offset_y:.3f}) scale({scale})">{body}</g></svg>\n')


for slug in DOCUMENTS:
    for small in (False, True):
        with open(f'{out}/{slug}-document{"-small" if small else ""}-macos.svg', 'w') as file:
            file.write(document(APPS[slug], small))

for slug, parts in APPS.items():
    for macos in (False, True):
        for small in (False, True):
            name = f'{out}/{slug}{"-small" if small else ""}{"-macos" if macos else ""}.svg'
            with open(name, 'w') as file:
                file.write(svg(parts, macos, small))
