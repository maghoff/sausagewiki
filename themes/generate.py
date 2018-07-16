#!/usr/bin/env python3

import json, colorsys

colors = json.load(open("material-colors.json", "r"))
palettes = colors['palettes']

def hex_to_rgb(h):
    return tuple(int(h[i:i+2], 16) / 255 for i in (0, 2 ,4))

def to_linear(x):
    if x < 0.04045:
        return x / 12.92
    else:
        return pow((x + 0.055) / 1.055, 2.4)

def rgb_to_linear(rgb):
    return [to_linear(x) for x in rgb]

def luma(rgb):
    r, g, b = rgb_to_linear(rgb)
    return (0.2126*r + 0.7152*g + 0.0722*b)

def prep(x):
    cols = [x['colors'][5], x['colors'][7]]
    rgb = [hex_to_rgb(c[1:]) for c in cols]
    brightness = [luma(c) for c in rgb]

    main_index = 0 if brightness[0] < 0.6 else 1
    dark_main = brightness[main_index] < 0.5

    return {
        "name": x['shade'].lower().replace(' ', '-'),
        "main": cols[main_index],
        "input": x['colors'][3 if main_index == 0 else 5],
        "text": "white" if dark_main else "black",
        "link": blues[2] if dark_main else blues[7],
    }

blues = [x for x in palettes if x['shade'] == "Blue"][0]["colors"]

themes = [prep(palette) for palette in palettes]

print(
    "\n".join(
        "\
.theme-{name} {{\n\
    --theme-main: {main};\n\
    --theme-text: {text};\n\
    --theme-input: {input};\n\
    --theme-link: {link};\n\
}}\n".format(**x)
        for x in themes
    )
)

# print("[" + ', '.join('"'+x['name']+'"' for x in themes) + "]")
