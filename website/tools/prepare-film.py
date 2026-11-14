"""Prepares the film for the website: its data, frames, stills and video.

    python website/tools/prepare-film.py --web-json WEB.json [FRAMES]
        [--video FILM.mp4] [--small-video FILM-SMALL.mp4]
        [--data website/film.json] [--out website/media/film]
        [--zip website-film.zip]

WEB.json is the data file that the film's assembly writes beside the film
(film/web.json): its frame rate, its frames, every shot with its seconds,
and every line with its moments and how it is set. It is the only source
of the film's numbers. From it this tool first writes the numbers of
website/film.json (the length of the film, the seconds of each chapter, the
lines) and then the parts of index.html made from them, through
write-page.mjs. Everything else in film.json is the site's own and is
kept: the name and the text of each chapter, and the shot it begins with.
A shot that begins no chapter belongs to the chapter before it, so a new
shot in the film lengthens a chapter without any change here.

FRAMES is a folder of PNG pictures of the film without its lines, one for
each frame, named by their number (0001.png, 0002.png, ...). Without it,
only the data and the page are written. Run the tool from the root of the
repository with Python 3 and Pillow, and Node.js for the page. Nothing
else is needed: no ffmpeg, no packages.

With FRAMES it writes into the output folder, which git ignores:

- manifest.json: what the page needs to know about everything below;
- large/pack-NN.bin and small/pack-NN.bin: every second frame of the film
  (12 a second) as WebP pictures, 1920 and 1280 pixels wide, laid end to end
  in a few dozen files. The manifest says where each picture starts and
  how long it is. The first packs hold every sixteenth frame of the film,
  so that the page can scrub through the whole film once they are in;
  the later packs fill in between;
- stills/<chapter>.webp: the frame stillAfter seconds into each chapter,
  shown before any frame has loaded;
- the video files, copied, if they are given.

With --zip it also packs the output folder into one file, the one the
publishing workflow downloads from the release (see website/README.md).
"""
import argparse
import io
import json
import os
import shutil
import subprocess
import sys
import time
import zipfile

from PIL import Image

# Every second frame of a film at 24 frames a second: 12 a second.
STEP = 2
# The levels of detail, as the step between pictures of the page's own
# numbering (12 a second): every 8th is every 16th frame of the film.
LEVELS = (8, 4, 2, 1)
# The large set is as wide as the film itself, so that nothing is lost on a
# wide screen; the small one is for phones, whose screens have about 1200
# pixels across, and for readers who save data. (They were 1280 and 640
# until the owner found the film soft on the site.)
SETS = {
    'large': {'width': 1920, 'quality': 80},
    'small': {'width': 1280, 'quality': 76},
}
# A pack is closed when it holds this many bytes.
PACK_BYTES = 1_500_000


def level_of(index):
    """The first level that contains a picture: 0 is the coarsest."""
    for level, step in enumerate(LEVELS):
        if index % step == 0:
            return level
    return len(LEVELS) - 1


def encode(image, width, quality):
    height = round(image.height * width / image.width)
    picture = image.resize((width, height), Image.LANCZOS) if image.width != width else image
    buffer = io.BytesIO()
    picture.save(buffer, 'WEBP', quality=quality, method=4)
    return buffer.getvalue(), height


TYPES = {'Instrument Sans': 'sans', 'Instrument Serif': 'serif', 'IBM Plex Mono': 'mono'}
# The colours of the lines, as the tokens of the site name them.
TONES = {'#e8e0d0': 'ink', '#b9af9d': 'soft', '#c9a96e': 'brass'}


def number(value):
    """11.0 is written 11, so that the page and the data read alike."""
    return int(value) if float(value).is_integer() else round(value, 3)


def write_film_data(web, data_path):
    """Writes the film's numbers from WEB.json into film.json."""
    with open(data_path, encoding='utf-8') as file:
        data = json.load(file)
    film = data['film']
    film['framesPerSecond'] = web['fps']
    film['frames'] = web['frames']
    film['seconds'] = number(web['seconds'])

    starts = {shot['shot']: shot['from_second'] for shot in web['shots']}
    for chapter in data['chapters']:
        if chapter['fromShot'] not in starts:
            sys.exit(f'Chapter "{chapter["id"]}" begins with shot {chapter["fromShot"]}, which the film does not have')
    chapters = sorted(data['chapters'], key=lambda chapter: starts[chapter['fromShot']])
    for index, chapter in enumerate(chapters):
        chapter['start'] = number(starts[chapter['fromShot']])
        following = chapters[index + 1]['fromShot'] if index + 1 < len(chapters) else None
        chapter['end'] = number(starts[following] if following else web['seconds'])
    if chapters[0]['start'] != 0:
        sys.exit('No chapter begins with the first shot of the film')
    for shot in web['shots']:
        if shot['shot'] not in {chapter['fromShot'] for chapter in chapters}:
            owner = [c for c in chapters if c['start'] <= shot['from_second'] < c['end']][0]
            print(f'Shot {shot["shot"]} begins no chapter; it is in "{owner["title"]}"')
    data['chapters'] = chapters

    lines = []
    for line in web['lines']:
        kind = TYPES.get(line['type'])
        tone = TONES.get(line['colour'].lower())
        if not kind or not tone:
            sys.exit(f'"{line["text"]}" is set in {line["type"]}, {line["colour"]}, which the site does not know')
        lines.append({
            'text': line['text'],
            'start': number(line['first_second']),
            'end': None if line['stays_to_the_end'] else number(line['last_second']),
            'type': kind,
            'weight': line['weight'],
            'size': line['size_of_height'],
            'y': line['centre_y_of_height'],
            'tone': tone,
        })
    data['lines'] = lines

    with open(data_path, 'w', encoding='utf-8', newline='\n') as file:
        json.dump(data, file, indent=2, ensure_ascii=False)
        file.write('\n')
    print(f'Wrote {data_path}: {film["seconds"]} s, {len(chapters)} chapters, {len(lines)} lines')

    page_tool = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'write-page.mjs')
    result = subprocess.run(['node', page_tool], check=False)
    if result.returncode:
        sys.exit('write-page.mjs failed: the page is not up to date with film.json')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('frames', nargs='?')
    parser.add_argument('--web-json', required=True)
    parser.add_argument('--video')
    parser.add_argument('--small-video')
    parser.add_argument('--data', default=os.path.join('website', 'film.json'))
    parser.add_argument('--out', default=os.path.join('website', 'media', 'film'))
    parser.add_argument('--zip')
    arguments = parser.parse_args()

    with open(arguments.web_json, encoding='utf-8') as file:
        web = json.load(file)
    data = write_film_data(web, arguments.data)
    if not arguments.frames:
        return

    film = data['film']
    if film['framesPerSecond'] % STEP:
        sys.exit(f'{film["framesPerSecond"]} frames a second cannot be halved')
    count = film['frames'] // STEP

    def source_path(index):
        # Picture `index` of the page is frame index * STEP + 1 of the film.
        return os.path.join(arguments.frames, f'{index * STEP + 1:04d}.png')

    missing = [i for i in range(count) if not os.path.isfile(source_path(i))]
    if missing:
        sys.exit(f'{len(missing)} frames are missing, the first {source_path(missing[0])}')
    for video in (arguments.video, arguments.small_video):
        if video and not os.path.isfile(video):
            sys.exit(f'There is no video at {video}')

    if os.path.isdir(arguments.out):
        shutil.rmtree(arguments.out)
    os.makedirs(arguments.out)

    started = time.monotonic()
    encoded = {name: [None] * count for name in SETS}
    heights = {}
    for index in range(count):
        image = Image.open(source_path(index)).convert('RGB')
        for name, settings in SETS.items():
            encoded[name][index], heights[name] = encode(image, settings['width'], settings['quality'])
        if index % 100 == 0:
            print(f'  picture {index + 1} of {count}', flush=True)
    print(f'Encoded {count} pictures in each of {len(SETS)} sizes in {time.monotonic() - started:.0f} s')

    manifest = {
        'version': 1,
        'framesPerSecond': film['framesPerSecond'] // STEP,
        'frameCount': count,
        'type': 'image/webp',
        'sets': {},
        'stills': {},
        'video': {},
    }

    # Packs: level by level, from the coarsest, and within a level in the
    # order of the film. The page loads the coarsest level first and then
    # what lies nearest the reader.
    for name, settings in SETS.items():
        folder = os.path.join(arguments.out, name)
        os.makedirs(folder)
        packs = []
        for level in range(len(LEVELS)):
            indices = [i for i in range(count) if level_of(i) == level]
            current, frames, offset = [], [], 0
            for index in indices:
                data_bytes = encoded[name][index]
                frames.append([index, offset, len(data_bytes)])
                current.append(data_bytes)
                offset += len(data_bytes)
                if offset >= PACK_BYTES or index == indices[-1]:
                    file_name = f'{name}/pack-{len(packs):02d}.bin'
                    with open(os.path.join(arguments.out, file_name), 'wb') as file:
                        file.write(b''.join(current))
                    packs.append({'file': file_name, 'level': level, 'bytes': offset, 'frames': frames})
                    current, frames, offset = [], [], 0
        total = sum(pack['bytes'] for pack in packs)
        coarse = sum(pack['bytes'] for pack in packs if pack['level'] == 0)
        manifest['sets'][name] = {
            'width': settings['width'],
            'height': heights[name],
            'bytes': total,
            'packs': packs,
        }
        print(f'{name}: {len(packs)} packs, {total / 1e6:.1f} MB in all, '
              f'{coarse / 1e6:.1f} MB for every 16th frame')

    # A still for every chapter, from the frame film.json names.
    stills = os.path.join(arguments.out, 'stills')
    os.makedirs(stills)
    for chapter in data['chapters']:
        # stillAfter is counted from the start of the chapter, so that the
        # still stays in its chapter when the film around it changes.
        moment = min(chapter['start'] + chapter['stillAfter'], chapter['end'] - 1 / film['framesPerSecond'])
        frame = int(moment * film['framesPerSecond']) + 1
        path = os.path.join(arguments.frames, f'{frame:04d}.png')
        if not os.path.isfile(path):
            continue
        image = Image.open(path).convert('RGB')
        if image.width > 1920:
            image = image.resize((1920, round(image.height * 1920 / image.width)), Image.LANCZOS)
        image.save(os.path.join(stills, f'{chapter["id"]}.webp'), 'WEBP', quality=84, method=6)
        manifest['stills'][chapter['id']] = f'stills/{chapter["id"]}.webp'

    for key, source in (('large', arguments.video), ('small', arguments.small_video)):
        if source:
            target = film['video'] if key == 'large' else film['smallVideo']
            shutil.copyfile(source, os.path.join(arguments.out, target))
            manifest['video'][key] = target
            print(f'video {key}: {target}, {os.path.getsize(source) / 1e6:.1f} MB')

    with open(os.path.join(arguments.out, 'manifest.json'), 'w', encoding='utf-8') as file:
        json.dump(manifest, file, separators=(',', ':'))
    print(f'Wrote {arguments.out}')

    if arguments.zip:
        with zipfile.ZipFile(arguments.zip, 'w', zipfile.ZIP_STORED) as archive:
            for folder, _, files in os.walk(arguments.out):
                for file_name in files:
                    path = os.path.join(folder, file_name)
                    archive.write(path, os.path.relpath(path, arguments.out))
        print(f'Wrote {arguments.zip}, {os.path.getsize(arguments.zip) / 1e6:.1f} MB')


if __name__ == '__main__':
    main()
