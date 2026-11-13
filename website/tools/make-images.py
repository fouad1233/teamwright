"""Makes the pictures of the website from pictures that are already in git.

    python website/tools/make-images.py

Run it from the root of the repository, with Python 3 and Pillow. It reads
the screenshots of the program (docs/screenshots, the dark ones), the key
visual (docs/design/key-visual.png), the mark of the program
(apps/desktop/public/teamwright.svg and the icons of the desktop shell) and
the fonts of the website, and writes into website/images:

- every screenshot the page shows, 960 and 1600 pixels wide, as WebP, each
  file under the budget of website/tools/check.mjs;
- the picture that a link to the site shows when it is shared, 1200 by 630;
- the icons of the site.

Nothing of the film is read here. The film is not in git and its pictures
are written by prepare-film.py into website/media/film, which git ignores.
"""
import io
import os
import shutil
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SCREENSHOTS = os.path.join(ROOT, 'docs', 'screenshots')
KEY_VISUAL = os.path.join(ROOT, 'docs', 'design', 'key-visual.png')
MARK_SVG = os.path.join(ROOT, 'apps', 'desktop', 'public', 'teamwright.svg')
SHELL_ICON = os.path.join(ROOT, 'apps', 'desktop', 'src-tauri', 'icons', 'icon.png')
WEBSITE = os.path.join(ROOT, 'website')
IMAGES = os.path.join(WEBSITE, 'images')
FONTS = os.path.join(WEBSITE, 'fonts')

# The screenshots the page shows. Dark only: the site is the dark room.
PICTURES = [
    'first-run', 'advisor-interview', 'advisor-proposal', 'advisor-board',
    'board', 'sprint-board', 'backlog', 'list', 'gantt', 'calendar',
    'dashboard', 'area', 'task', 'team', 'office', 'specialist-working',
    'specialist-office', 'specialist-report', 'specialist-inbox',
]
WIDTHS = (960, 1600, 2400)
# The largest a picture of the site may be. check.mjs holds the same number.
BUDGET_BYTES = 150 * 1024

# Colours of the tokens (apps/desktop/src/shared/styles/tokens.css), at night.
GROUND = (0x12, 0x10, 0x0E)
INK = (0xE8, 0xE0, 0xD0)
INK_SOFT = (0xB9, 0xAF, 0x9D)
INK_FAINT = (0x9A, 0x90, 0x7F)
BRASS = (0xC9, 0xA9, 0x6E)
RULE = (0x2E, 0x2A, 0x25)


def save_within_budget(image, path):
    """Saves as WebP at the best quality that stays under the budget."""
    for quality in (82, 76, 70, 64, 58, 52, 46):
        buffer = io.BytesIO()
        image.save(buffer, 'WEBP', quality=quality, method=6)
        if buffer.tell() <= BUDGET_BYTES:
            with open(path, 'wb') as file:
                file.write(buffer.getvalue())
            return quality, buffer.tell()
    raise SystemExit(f'{path}: over {BUDGET_BYTES} bytes even at the lowest quality')


def make_pictures():
    os.makedirs(IMAGES, exist_ok=True)
    for name in PICTURES:
        source = Image.open(os.path.join(SCREENSHOTS, f'{name}-dark.png')).convert('RGB')
        for width in WIDTHS:
            height = round(source.height * width / source.width)
            picture = source.resize((width, height), Image.LANCZOS)
            path = os.path.join(IMAGES, f'{name}-{width}.webp')
            quality, size = save_within_budget(picture, path)
            print(f'{os.path.relpath(path, ROOT)}  {width}x{height}  q{quality}  {size // 1024} kB')


def font(file, size, weight=None):
    face = ImageFont.truetype(os.path.join(FONTS, file), size)
    if weight is not None:
        try:
            face.set_variation_by_axes([weight])
        except OSError:
            pass
    return face


def make_share_picture():
    """The picture a shared link shows: the key visual, a still of the
    program, the name and the promise, on the ground of the dark room."""
    width, height = 1200, 630
    sheet = Image.new('RGB', (width, height), GROUND)
    draw = ImageDraw.Draw(sheet)
    margin = 48

    # The key visual, as a plate on the left.
    plate = Image.open(KEY_VISUAL).convert('RGB')
    plate_width = 600
    plate = plate.resize((plate_width, round(plate.height * plate_width / plate.width)), Image.LANCZOS)
    sheet.paste(plate, (margin, margin))

    # A still of the program to its right, cut from the board, with a
    # hairline round it.
    still = Image.open(os.path.join(SCREENSHOTS, 'board-dark.png')).convert('RGB')
    still_left = margin + plate_width + 24
    still_width = width - margin - still_left
    still_height = plate.height
    scale = 0.62
    still = still.resize((round(still.width * scale), round(still.height * scale)), Image.LANCZOS)
    # The navigation of the program ends 220 pixels in at this scale.
    still = still.crop((226, 0, 226 + still_width, still_height))
    sheet.paste(still, (still_left, margin))
    draw.rectangle(
        (still_left - 1, margin - 1, still_left + still_width, margin + still_height),
        outline=RULE,
    )

    # The title block at the foot.
    baseline = margin + plate.height + 44
    name = font('instrument-sans-latin-wght-normal.woff2', 42, 600)
    draw.text((margin, baseline), 'Teamwright', font=name, fill=INK, anchor='ls')
    draw.text(
        (margin + draw.textlength('Teamwright', font=name) + 22, baseline),
        'You manage. A right-sized team of AI specialists does the work.',
        font=font('instrument-serif-latin-400-normal.woff2', 30),
        fill=INK_SOFT,
        anchor='ls',
    )
    draw.line((margin, baseline + 22, width - margin, baseline + 22), fill=RULE, width=1)
    label = font('ibm-plex-mono-latin-400-normal.woff2', 15)
    draw.text((margin, baseline + 48), 'OPEN SOURCE  ·  APACHE 2.0  ·  A DESKTOP APP', font=label, fill=INK_FAINT, anchor='ls')
    draw.text((width - margin, baseline + 48), 'FOUAD1233.GITHUB.IO/TEAMWRIGHT', font=label, fill=BRASS, anchor='rs')

    path = os.path.join(IMAGES, 'share.png')
    sheet.save(path, 'PNG', optimize=True)
    size = os.path.getsize(path)
    if size > BUDGET_BYTES:
        # A JPEG of the same picture is much smaller; social sites read both.
        os.remove(path)
        path = os.path.join(IMAGES, 'share.jpg')
        sheet.save(path, 'JPEG', quality=88, optimize=True, progressive=True)
        size = os.path.getsize(path)
    print(f'{os.path.relpath(path, ROOT)}  {width}x{height}  {size // 1024} kB')


def make_icons():
    icons = os.path.join(IMAGES, 'icons')
    os.makedirs(icons, exist_ok=True)
    shutil.copyfile(MARK_SVG, os.path.join(icons, 'mark.svg'))
    source = Image.open(SHELL_ICON).convert('RGBA')
    for size, name in ((32, 'icon-32.png'), (180, 'apple-touch-icon.png'), (192, 'icon-192.png'), (512, 'icon-512.png')):
        icon = source if size == source.width else source.resize((size, size), Image.LANCZOS)
        if name == 'apple-touch-icon.png':
            # A touch icon has no transparency: the corners take the ground.
            flat = Image.new('RGB', icon.size, GROUND)
            flat.paste(icon, (0, 0), icon)
            icon = flat
        icon.save(os.path.join(icons, name), 'PNG', optimize=True)
    source.resize((32, 32), Image.LANCZOS).save(os.path.join(WEBSITE, 'favicon.ico'), sizes=[(16, 16), (32, 32)])
    print('website/images/icons/ and website/favicon.ico')


def main():
    if not os.path.isfile(os.path.join(FONTS, 'instrument-sans-latin-wght-normal.woff2')):
        sys.exit('The fonts of the website are missing: website/fonts')
    make_pictures()
    make_share_picture()
    make_icons()


if __name__ == '__main__':
    main()
