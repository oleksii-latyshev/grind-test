"""Draw the graduation cap used as the app icon and the menu-bar tray icon.

Two renders from one geometry so the tray icon is unmistakably the same mark as the one in
the Dock: a filled version on the app's primary colour, and a flat black-on-transparent
template that macOS recolours to follow the menu bar.
"""

from PIL import Image, ImageDraw

PRIMARY = (0, 117, 149, 255)  # --primary, oklch(0.52 0.105 223.128)
CAP = (236, 254, 255, 255)  # --primary-foreground
SS = 4  # supersampling factor; everything below is drawn at SS× and downsampled


def cap_polygons(size):
    """Mortarboard geometry in a `size`×`size` box, as (polygon, kind) pairs."""
    s = size
    cx = s / 2

    # The board: a wide, shallow diamond seen at an angle.
    board_y = s * 0.36
    half_w = s * 0.40
    half_h = s * 0.155
    board = [
        (cx, board_y - half_h),
        (cx + half_w, board_y),
        (cx, board_y + half_h),
        (cx - half_w, board_y),
    ]

    # The head-piece below it, drawn as a cup hanging under the board's front point.
    cup_top = board_y + half_h * 0.30
    cup_w = s * 0.215
    cup_bottom = s * 0.70
    cup = [
        (cx - cup_w, cup_top),
        (cx + cup_w, cup_top),
        (cx + cup_w * 0.82, cup_bottom),
        (cx + cup_w * 0.45, cup_bottom + s * 0.045),
        (cx - cup_w * 0.45, cup_bottom + s * 0.045),
        (cx - cup_w * 0.82, cup_bottom),
    ]

    # Tassel: a cord from the board's right corner, then the knot.
    cord_x = cx + half_w * 0.80
    cord = [
        (cord_x - s * 0.018, board_y - half_h * 0.10),
        (cord_x + s * 0.018, board_y - half_h * 0.10),
        (cord_x + s * 0.018, s * 0.60),
        (cord_x - s * 0.018, s * 0.60),
    ]
    knot = (cord_x - s * 0.042, s * 0.585, cord_x + s * 0.042, s * 0.72)

    return board, cup, cord, knot


def draw_cap(draw, size, fill, hole):
    """The cap, with the cup punched away from the board so the mark reads at 16px."""
    board, cup, cord, knot = cap_polygons(size)
    draw.polygon(cup, fill=fill)
    draw.polygon(cord, fill=fill)
    draw.ellipse(knot, fill=fill)
    draw.polygon(board, fill=fill)
    # A sliver of background between board and cup keeps the two shapes legible when the
    # whole thing is 16 pixels across.
    draw.line([(size * 0.5 - size * 0.235, size * 0.475), (size * 0.5 + size * 0.235, size * 0.475)],
              fill=hole, width=max(1, int(size * 0.022)))


def app_icon(size=1024):
    big = size * SS
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    # macOS and Windows both round the corners themselves on some surfaces; a squircle-ish
    # radius looks right either way.
    draw.rounded_rectangle([0, 0, big - 1, big - 1], radius=int(big * 0.225), fill=PRIMARY)
    draw_cap(draw, big, CAP, PRIMARY)
    return image.resize((size, size), Image.LANCZOS)


def tray_icon(size):
    big = size * SS
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    # Black on transparent: a macOS template image, recoloured by the system for light and
    # dark menu bars. Inset, because the menu bar gives the icon less room than the Dock.
    pad = big * 0.08
    inner = Image.new("RGBA", (int(big - 2 * pad), int(big - 2 * pad)), (0, 0, 0, 0))
    inner_draw = ImageDraw.Draw(inner)
    draw_cap(inner_draw, inner.width, (0, 0, 0, 255), (0, 0, 0, 0))
    image.paste(inner, (int(pad), int(pad)), inner)
    del draw
    return image.resize((size, size), Image.LANCZOS)


if __name__ == "__main__":
    import sys

    out = sys.argv[1]
    app_icon(1024).save(f"{out}/app-icon.png")
    tray_icon(32).save(f"{out}/tray.png")
    tray_icon(64).save(f"{out}/tray@2x.png")
    print("written")
