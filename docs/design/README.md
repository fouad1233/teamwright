# Design

Teamwright has one visual language for everything it shows: the interface,
the logo, and the pictures that explain it.

| File | What it is |
|---|---|
| [philosophy.md](philosophy.md) | *Joinery*, the design philosophy. Read this first. |
| [key-visual.png](key-visual.png) | *Plate 01*, the key visual: the philosophy expressed on one sheet. |
| [`tools/design/key-visual.mjs`](../../tools/design/key-visual.mjs) | The program that draws the plate. |

![Plate 01: an assembly of five parts, and the order of the work](key-visual.png)

## The language in short

- **Parts that fit.** Rectangles on one module, meeting along true lines with
  a constant gap between them. Hairline rules instead of shadows.
- **Paper, graphite and ink.** Warm neutrals for the ground and the text.
- **One metal.** Brass marks the single point where the parts are held: in
  the logo it is the keystone, in the interface it is where you are.
- **Pigment for meaning only.** Color tells statuses, priorities and people
  apart. Nothing is colored for decoration.
- **Three typefaces.** Instrument Sans for the interface, Instrument Serif
  for a single gesture per view, IBM Plex Mono for anything that is counted
  or coded. All three are open fonts under the SIL Open Font License.

## How the plate reads

*Figure 1* is the logo taken apart: five parts make a T. Three hatched parts
are the specialists, the ink part between them is the Advisor, and the brass
keystone on top is the manager, who holds the others in place.

*Figure 2* is a plan: bars on a scale, each beginning where the bars it waits
for end. The brass diamonds are the moments where the decision is the
manager's.

## Drawing the plate again

```bash
npm install --no-save @resvg/resvg-js
node tools/design/key-visual.mjs <fonts-directory> docs/design
```

`<fonts-directory>` is a folder with the `.ttf` files of Instrument Sans,
Instrument Serif and IBM Plex Mono.
