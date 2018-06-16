#!/usr/bin/env python3

import json, colorsys

colors = json.load(open("material-colors.json", "r"))
palettes = colors['palettes']

def hex_to_rgb(h):
    return tuple(int(h[i:i+2], 16) / 255 for i in (0, 2 ,4))

def prep(x):
    cols = [x['colors'][5], x['colors'][7]]
    rgb = [hex_to_rgb(c[1:]) for c in cols]
    hls = [colorsys.rgb_to_hls(*c) for c in rgb]

    main_index = 0 if hls[0][1] < 0.6 else 1
    dark_main = hls[main_index][1] < 0.6

    return {
        "name": x['shade'].lower().replace(' ', '-'),
        "main": cols[main_index],
        "input": x['colors'][3 if main_index == 0 else 5],
        "text": "white" if dark_main else "black",
    }

print(
    "\n".join(
        "\
.theme-{name} {{\n\
    --theme-main: {main};\n\
    --theme-text: {text};\n\
    --theme-input: {input};\n\
    --theme-link: #01579b;\n\
}}\n".format(**x)
        for x in (prep(palette) for palette in palettes)
    )
)
