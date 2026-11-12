/*
 * The pixels of the people in the office and of what stands around them
 * (decision 0019). Every drawing is a grid of letters, one letter a pixel,
 * drawn here by hand. A dot is empty. The line round a figure is not drawn:
 * spriteModel.ts adds it, so that every figure has the same edge.
 *
 * The letters name what a pixel is, not its color; paint.ts gives each its
 * color for a character and a theme:
 *
 *   s z   skin, and its shadow          e     eyes
 *   h j   hair, and its shadow
 *   c d   the shirt, and its shadow     n     the strip down the front
 *   b B   the lower half of the body    l L   the sleeves
 *   p q   trousers, near and far leg    k     shoes
 *   1 2   what a role wears on its head, and its shadow
 *   m M i metal, its shadow, its light  x X   black, and its shadow
 *   w v t paper, its shadow, writing    g G   a circuit board
 *   o O T wood, its shadow, its top     y Y   light from a screen
 *   f     steam and smoke               r R   red
 *   u     coffee                        a A   leaves
 *   $     the gold of the theme         c     (on a prop) the member's color
 *
 * A figure is 16 pixels wide and 24 high. Its feet stand on row 22; the
 * last row and the first stay empty for the outline. The head is a third
 * of the figure, so that a face and what is worn on it read at a glance.
 */

/** A drawing placed in the frame of a figure: its top left corner, and its rows. */
export interface Part {
  x: number;
  y: number;
  rows: readonly string[];
}

/** The width and height of the frame a figure is drawn in. */
export const FIGURE_WIDTH = 16;
export const FIGURE_HEIGHT = 24;
/** The row the feet stand on. */
export const FEET_ROW = 22;
/**
 * The row where the far edge of a bench cuts a seated figure: what lies
 * below it stands on the bench.
 */
export const DESK_ROW = 17;

/** The head, as seen from the front, from behind and from the right. */
export const HEADS = {
  front: {
    x: 3,
    y: 2,
    rows: [
      '..ssssss..',
      '.ssssssss.',
      'ssssssssss',
      'ssssssssss',
      'ssessssess',
      'ssessssess',
      'ssssssssss',
      '.zssssssz.',
      '..zzzzzz..',
    ],
  },
  /** Looking down at the work on the bench. */
  down: {
    x: 3,
    y: 2,
    rows: [
      '..ssssss..',
      '.ssssssss.',
      'ssssssssss',
      'ssssssssss',
      'ssssssssss',
      'ssessssess',
      'ssssssssss',
      '.zssssssz.',
      '..zzzzzz..',
    ],
  },
  /** Looking a little to the side, now and then, while idle. */
  glance: {
    x: 3,
    y: 2,
    rows: [
      '..ssssss..',
      '.ssssssss.',
      'ssssssssss',
      'ssssssssss',
      'sssesssses',
      'sssesssses',
      'ssssssssss',
      '.zssssssz.',
      '..zzzzzz..',
    ],
  },
  back: {
    x: 3,
    y: 2,
    rows: [
      '..ssssss..',
      '.ssssssss.',
      'ssssssssss',
      'ssssssssss',
      'ssssssssss',
      'ssssssssss',
      'ssssssssss',
      '.zssssssz.',
      '..zzzzzz..',
    ],
  },
  side: {
    x: 3,
    y: 2,
    rows: [
      '..ssssss..',
      '.ssssssss.',
      'ssssssssss',
      'ssssssssss',
      'sssssssess',
      'sssssssess',
      'ssssssssss',
      '.zsssssss.',
      '..zzzzz...',
    ],
  },
} as const satisfies Record<string, Part>;

/** The ways hair is worn. Each is drawn from the front, from behind, and from the right. */
export const HAIR_STYLES = ['short', 'parted', 'long', 'tied', 'curly'] as const;
export type HairStyle = (typeof HAIR_STYLES)[number];

/** The way a figure faces: towards the room, away from it, or to the right. */
export type Facing = 'front' | 'back' | 'side';

const HAIR_ORIGIN = { x: 2, y: 1 };

export const HAIR: Readonly<Record<HairStyle, Readonly<Record<Facing, Part>>>> = {
  short: {
    front: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhj..h.',
        '.h........h.',
      ],
    },
    back: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.jhhhhhhhhj.',
        '..jjjjjjjj..',
      ],
    },
    side: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhh....',
        '.hhhh.......',
        '.hhhh.......',
        '.jhhj.......',
        '..jj........',
      ],
    },
  },
  parted: {
    front: {
      ...HAIR_ORIGIN,
      rows: [
        '....hhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhj.',
        '.h..jhhhhhh.',
        '.h........h.',
      ],
    },
    back: {
      ...HAIR_ORIGIN,
      rows: [
        '....hhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.jhhhhhhhhj.',
        '..jjjjjjjj..',
      ],
    },
    side: {
      ...HAIR_ORIGIN,
      rows: [
        '....hhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhj.',
        '.hhhhhhj....',
        '.hhhh.......',
        '.hhhh.......',
        '.jhhj.......',
        '..jj........',
      ],
    },
  },
  long: {
    front: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        'hhhhhhhhhhhh',
        'hhhhj..jhhhh',
        'hhh......hhh',
        'hh........hh',
        'hh........hh',
        'hh........hh',
        'hj........jh',
        'hj........jh',
        '.j........j.',
      ],
    },
    back: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        'hhhhhhhhhhhh',
        '.jhhhhhhhhj.',
        '..jjjjjjjj..',
      ],
    },
    side: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhh....',
        '.hhhhh......',
        '.hhhhh......',
        '.hhhhh......',
        '.hhhhh......',
        '.hhhhj......',
        '.hhhj.......',
        '.jjj........',
      ],
    },
  },
  tied: {
    front: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hj......jhh',
        '.h........hh',
        '..........hh',
        '..........hj',
        '..........hj',
        '..........jj',
        '...........j',
      ],
    },
    back: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.hhhhhhhhhh.',
        '.jhhhhhhhhj.',
        '..jjjhhjjj..',
        '.....hh.....',
        '.....hj.....',
        '.....jj.....',
        '......j.....',
      ],
    },
    side: {
      ...HAIR_ORIGIN,
      rows: [
        '...hhhhhh...',
        '..hhhhhhhh..',
        'hhhhhhhhhhh.',
        'hhhhhhhhhhh.',
        'jhhhhhhh....',
        'jhhhh.......',
        'j.jhh.......',
        'j...........',
      ],
    },
  },
  curly: {
    front: {
      ...HAIR_ORIGIN,
      rows: [
        '..hhjhhjhh..',
        '.hhhhhhhhhh.',
        'hhjhhhhhhjhh',
        'hhhhhhhhhhhh',
        'hhh.h..h.hhh',
        'hh........hh',
        'hh........hh',
        '.j........j.',
      ],
    },
    back: {
      ...HAIR_ORIGIN,
      rows: [
        '..hhjhhjhh..',
        '.hhhhhhhhhh.',
        'hhjhhhhhhjhh',
        'hhhhhhhhhhhh',
        'hhhhjhhjhhhh',
        'hhhhhhhhhhhh',
        'hjhhhhhhhhjh',
        '.hhjhhhhjhh.',
        '..jjjjjjjj..',
      ],
    },
    side: {
      ...HAIR_ORIGIN,
      rows: [
        '..hhjhhjhh..',
        '.hhhhhhhhhh.',
        'hhjhhhhhhjh.',
        'hhhhhhhhhhh.',
        'hhhhhhhhh...',
        'hhhhhh......',
        'hjhhhh......',
        '.hhhj.......',
        '..jj........',
      ],
    },
  },
};

/** The body below the head: shoulders, arms, legs and shoes. */
const BODY_TOP = 11;

const TORSO_FRONT = [
  '...lcccnnccdL...',
  '...lcccnnccdL...',
  '...lcccnnccdL...',
  '...lbbbnnbbBL...',
  '...lbbbnnbbBL...',
  '...sbbbbbbbBs...',
];

const TORSO_SIDE = [
  '.....cccccd.....',
  '.....ccllcd.....',
  '.....ccllcd.....',
  '.....bbllbB.....',
  '.....bbllbB.....',
  '.....bbssbB.....',
];

const HIPS_FRONT = '....pppppppq....';
const LEG_FRONT = '....ppp..ppq....';
const HIPS_SIDE = '.....ppppq......';
const LEG_SIDE = '......ppq.......';

/** The bodies of every pose, each drawn from row 11 down. */
export const BODIES = {
  standFront: {
    x: 0,
    y: BODY_TOP,
    rows: [...TORSO_FRONT, HIPS_FRONT, LEG_FRONT, LEG_FRONT, LEG_FRONT, '....kkk..kkk....'],
  },
  /** A step with the left foot; the right is lifted. */
  walkFrontLeft: {
    x: 0,
    y: BODY_TOP,
    rows: [
      ...TORSO_FRONT,
      HIPS_FRONT,
      LEG_FRONT,
      LEG_FRONT,
      '....ppp..kkk....',
      '....kkk.........',
    ],
  },
  walkFrontRight: {
    x: 0,
    y: BODY_TOP,
    rows: [
      ...TORSO_FRONT,
      HIPS_FRONT,
      LEG_FRONT,
      LEG_FRONT,
      '....kkk..ppq....',
      '.........kkk....',
    ],
  },
  standSide: {
    x: 0,
    y: BODY_TOP,
    rows: [...TORSO_SIDE, HIPS_SIDE, LEG_SIDE, LEG_SIDE, LEG_SIDE, '......kkkk......'],
  },
  /** A stride, the near leg forward and the arm swinging forward. */
  walkSideNear: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '.....cccccd.....',
      '.....ccllcd.....',
      '.....ccclld.....',
      '.....bbbbls.....',
      '.....bbbbbB.....',
      '.....bbbbbB.....',
      HIPS_SIDE,
      '.....qq.pp......',
      '....qq...pp.....',
      '...qq.....pp....',
      '..kkk.....kkk...',
    ],
  },
  /** A stride, the far leg forward and the arm swinging back. */
  walkSideFar: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '.....cccccd.....',
      '.....cllccd.....',
      '.....llcccd.....',
      '....sbbbbbB.....',
      '.....bbbbbB.....',
      '.....bbbbbB.....',
      HIPS_SIDE,
      '.....pp.qq......',
      '....pp...qq.....',
      '...pp.....qq....',
      '..kkk.....kkk...',
    ],
  },
  /** Seated behind a bench, the hands resting on it. Row 17 is on the bench. */
  seatRest: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lbbbnnbbBL...',
      '....lbbnnbBL....',
      '....lbbbbbBL....',
      '....ss....ss....',
    ],
  },
  /** At work: both hands on the work. */
  seatWork: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lbbbnnbbBL...',
      '....lbbnnbBL....',
      '.....lbbbbL.....',
      '.....ss..ss.....',
    ],
  },
  /** At work: the right hand lifted. */
  seatWorkRight: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lbbbnnbbBL...',
      '....lbbnnbBL....',
      '.....lbbbbLs....',
      '.....ss.........',
    ],
  },
  /** At work: the left hand lifted. */
  seatWorkLeft: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lcccnnccdL...',
      '...lbbbnnbbBL...',
      '....lbbnnbBL....',
      '....slbbbbL.....',
      '.........ss.....',
    ],
  },
  /** Waiting for the manager: the right arm is raised. The arm itself is drawn over the head. */
  seatWait: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '...lcccnnccdLL..',
      '...lcccnnccd....',
      '...lcccnnccd....',
      '...lbbbnnbbB....',
      '....lbbnnbB.....',
      '....lbbbbbB.....',
      '....ss..........',
    ],
  },
  /** Idle: the right hand lifts a cup to the mouth. The arm and cup are drawn over the head. */
  seatSip: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '...lcccnnccdL...',
      '...lcccnnccd....',
      '...lcccnnccd....',
      '...lbbbnnbbB....',
      '....lbbnnbB.....',
      '....lbbbbbB.....',
      '....ss..........',
    ],
  },
  /** Idle: both arms up, a stretch. The arms are drawn over the head. */
  seatStretch: {
    x: 0,
    y: BODY_TOP,
    rows: [
      '..llcccnnccdLL..',
      '....cccnnccd....',
      '....cccnnccd....',
      '....bbbnnbbB....',
      '....bbbnnbbB....',
      '....bbbbbbbB....',
      '................',
    ],
  },
} as const satisfies Record<string, Part>;

export type BodyName = keyof typeof BODIES;

/** The raised arm of a figure that waits, drawn last so that it stands over the hair. */
export const RAISED_ARM: Part = {
  x: 12,
  y: 1,
  rows: ['.ss.', '.ss.', '.LL.', '.LL.', '.LL.', '.LL.', '.LL.', '.LL.', '.LL.', 'LL..'],
};

/** The left arm raised as well, for a stretch. */
export const RAISED_LEFT_ARM: Part = {
  x: 0,
  y: 1,
  rows: ['.ss.', '.ss.', '.ll.', '.ll.', '.ll.', '.ll.', '.ll.', '.ll.', '.ll.', '..ll'],
};

/** A cup lifted to the mouth in the right hand, its arm bent up from the shoulder. */
export const CUP_TO_MOUTH: Part = {
  x: 8,
  y: 7,
  rows: ['xxxx..', 'xwwvx.', 'xwwvss', '.xxxss', '....L.', '....L.'],
};

/** The top of a desk under the things on a bench: wood, with a rule of brass along its edge. */
export const DESK_TOP: Part = {
  x: 1,
  y: 17,
  rows: [
    '.TTTTTTTTTTTTTTTTTTTTTTTT.',
    'TTTTTTTTTTTTTTTTTTTTTTTTTT',
    'TTTTTTTTTTTTTTTTTTTTTTTTTT',
    '$$$$$$$$$$$$$$$$$$$$$$$$$$',
    'oooooooooooooooooooooooooo',
    'OOOOOOOOOOOOOOOOOOOOOOOOOO',
  ],
};

/** The parts of the body a role may dress differently. */
export type Garment = 'shirt' | 'labCoat' | 'apron' | 'cardigan' | 'vest' | 'jacket' | 'robe';

/** What a role wears on its head, from the front, from behind and from the right. */
export const HEADWEAR = {
  headphones: {
    front: {
      x: 2,
      y: 1,
      rows: [
        '...xxxxxx...',
        '..x......x..',
        '.x........x.',
        '.x........x.',
        'MM........MM',
        'Mm........mM',
        'MM........MM',
      ],
    },
    back: {
      x: 2,
      y: 1,
      rows: [
        '...xxxxxx...',
        '..x......x..',
        '.x........x.',
        '.x........x.',
        'MM........MM',
        'Mm........mM',
        'MM........MM',
      ],
    },
    side: {
      x: 2,
      y: 1,
      rows: [
        '...xxxxx....',
        '...x........',
        '...x........',
        '...x........',
        '..MMM.......',
        '..MmM.......',
        '..MMM.......',
      ],
    },
  },
  loupe: {
    front: { x: 3, y: 3, rows: ['...mmmm...', '...miim...', 'mmmmmmmmmm'] },
    back: { x: 3, y: 5, rows: ['mmmmmmmmmm'] },
    side: { x: 3, y: 3, rows: ['........mm', '........mi', 'mmmmmmmmmm'] },
  },
  beret: {
    front: { x: 2, y: 1, rows: ['...111111...', '.1111111111.', '11111111112.', '.222222222..'] },
    back: { x: 2, y: 1, rows: ['...111111...', '.1111111111.', '.21111111111', '..222222222.'] },
    side: { x: 2, y: 1, rows: ['..111111....', '1111111111..', '.22222222...'] },
  },
  glasses: {
    front: { x: 4, y: 6, rows: ['i.iiii.i', 'i.i..i.i', '.i....i.'] },
    back: { x: 3, y: 6, rows: [] },
    side: { x: 6, y: 6, rows: ['iiii.i', '...i.i', '....i.'] },
  },
} as const satisfies Record<string, Readonly<Record<Facing, Part>>>;

export type Headwear = keyof typeof HEADWEAR;

/*
 * What stands on a bench. A seated figure is drawn in a frame wider than a
 * figure, 28 by 24, with the figure on the left and its things in front of
 * it and to its right. Row 17 and below lie on the bench.
 */

export const SCENE_WIDTH = 28;

/** The back of the chair at a bench, which shows while its person is away. */
export const CHAIR: Part = {
  x: 4,
  y: 11,
  rows: ['..xxxxxx..', '.xXXXXXXx.', '.xXXXXXXx.', '.xXXXXXXx.', '.xxxxxxxx.', '..x....x..'],
};

/** A thing on a bench, with a second drawing for while work goes on, if it moves. */
export interface Prop {
  still: readonly Part[];
  /** Drawn over the still parts, one after the other, while the figure works. */
  working?: readonly (readonly Part[])[];
}

const MUG: Part = { x: 16, y: 15, rows: ['vuuv.', 'wwwwv', 'wwwwv', 'vvvv.'] };

export const PROPS = {
  /** A laptop, seen from behind its lid, with a mug beside it. */
  laptop: {
    still: [
      {
        x: 3,
        y: 13,
        rows: ['.mmmmmmmm.', '.mmmmmmmm.', '.mmmmmmmm.', '.mmmmmmmm.', '.MMMMMMMM.', 'iiiiiiiiii'],
      },
      MUG,
    ],
    // The light of the screen falls on whoever looks into it, and flickers.
    working: [
      [{ x: 4, y: 12, rows: ['yyyyyyyy'] }],
      [{ x: 4, y: 12, rows: ['yyYyyyyy'] }],
      [{ x: 4, y: 12, rows: ['yyyyyYyy'] }],
    ],
  },
  /** A circuit board, a soldering iron in its stand, and a wisp of smoke while it works. */
  solder: {
    still: [
      { x: 3, y: 18, rows: ['gggggggggg', 'gxxggxggxg', 'GGGGGGGGGG'] },
      { x: 9, y: 14, rows: ['..xx', '.xx.', 'mm..', 'i...'] },
      { x: 16, y: 13, rows: ['.m.m.', '.mmm.', '..m..', '.mxm.', 'MMMMM', 'xxxxx'] },
    ],
    working: [
      [{ x: 8, y: 14, rows: ['f'] }],
      [{ x: 7, y: 13, rows: ['f'] }],
      [{ x: 8, y: 12, rows: ['f.', '.f'] }],
    ],
  },
  /** A machined part and a caliper. */
  caliper: {
    still: [
      { x: 3, y: 16, rows: ['.mmmmm.', 'mmiiimm', 'mmmmmmm', 'MMMMMMM'] },
      { x: 11, y: 18, rows: ['iiiiiiiiii', 'm.........', 'm.........'] },
      { x: 18, y: 16, rows: ['.xx.', 'xmmx', '.xx.'] },
    ],
    // The slide of the caliper opens and closes on the part.
    working: [[{ x: 13, y: 18, rows: ['M', 'm', 'm'] }], [{ x: 15, y: 18, rows: ['M', 'm', 'm'] }]],
  },
  /** An open notebook and a pen, and a stack of books. */
  notebook: {
    still: [
      { x: 3, y: 18, rows: ['wwwwwvwwwww', 'wttwwvwttww', 'vvvvvvvvvvv'] },
      { x: 17, y: 13, rows: ['rrrrrr', 'RRRRRR', 'oooooo', 'OOOOOO', 'gggggg', 'GGGGGG'] },
    ],
    working: [
      [{ x: 10, y: 15, rows: ['.x', 'x.', 'r.'] }],
      [{ x: 9, y: 15, rows: ['.x', 'x.', 'r.'] }],
    ],
  },
  /** A drawing tablet with a stroke on it, and a fan of colors. */
  tablet: {
    still: [
      { x: 3, y: 17, rows: ['xxxxxxxxxx', 'xXXXXXXXXx', 'xxxxxxxxxx'] },
      { x: 17, y: 14, rows: ['r...', 'r$..', 'r$g.', 'r$gc', 'xxxx'] },
    ],
    working: [
      [{ x: 5, y: 18, rows: ['yy....'] }],
      [{ x: 5, y: 18, rows: ['yyyy..'] }],
      [{ x: 5, y: 18, rows: ['yyyyyy'] }],
    ],
  },
  /** An instrument with a trace on its screen, and a clipboard. */
  scope: {
    still: [
      {
        x: 16,
        y: 11,
        rows: ['mmmmmmmm', 'mxxxxxxm', 'mxxxxxxm', 'mxxxxxxm', 'mmmmmmmm', 'mmiimiim', 'MMMMMMMM'],
      },
      { x: 3, y: 18, rows: ['..xxx..', 'wwwwwww', 'wtwtttw', 'vvvvvvv'] },
    ],
    working: [
      [{ x: 17, y: 12, rows: ['a....a', '.a..a.', '..aa..'] }],
      [{ x: 17, y: 12, rows: ['..aa..', '.a..a.', 'a....a'] }],
    ],
  },
  /** Papers under a magnifying glass, and a small chart. */
  papers: {
    still: [
      { x: 3, y: 18, rows: ['wwwwwwwwww', 'wtttwwtttw', 'vvvvvvvvvv'] },
      { x: 9, y: 15, rows: ['.mm.', 'miim', '.mm.', '..x.'] },
      {
        x: 17,
        y: 12,
        rows: ['wwwwww', 'wwwwgw', 'wwrwgw', 'w$rwgw', 'w$rwgw', 'vvvvvv', '.o..o.'],
      },
    ],
    // The lines are marked as they are read.
    working: [[{ x: 4, y: 19, rows: ['ttt'] }], [{ x: 7, y: 19, rows: ['ttt'] }]],
  },
  /** The plan of the project spread out, and rolled plans beside it. */
  plan: {
    still: [
      { x: 2, y: 18, rows: ['wwwwwwwwwwwww', 'wt$tttwt$tttw', 'vvvvvvvvvvvvv'] },
      { x: 16, y: 15, rows: ['...ww', '..wwv', '.wwv.', 'wwv..', 'wv...'] },
    ],
    // A pencil draws on the plan.
    working: [[{ x: 6, y: 16, rows: ['.x', 'r.'] }], [{ x: 9, y: 16, rows: ['.x', 'r.'] }]],
  },
  /** A tray of letters, and a mug. */
  tray: {
    still: [
      { x: 3, y: 18, rows: ['wwwwwwwww', 'wtwttwwtw', 'vvvvvvvvv'] },
      { x: 17, y: 14, rows: ['.www.', 'owwwo', 'ooooo', 'OOOOO'] },
      { ...MUG, x: 12, y: 16 },
    ],
  },
  /** Papers and a mug. */
  mug: {
    still: [{ x: 3, y: 18, rows: ['wwwwwwww', 'wttwtttw', 'vvvvvvvv'] }, MUG],
    // Steam rises from the mug.
    working: [[{ x: 17, y: 13, rows: ['f.', '.f'] }], [{ x: 17, y: 13, rows: ['.f', 'f.'] }]],
  },
} as const satisfies Record<string, Prop>;

export type PropName = keyof typeof PROPS;

/*
 * The shared places of the room. Each is drawn as a piece of furniture in
 * front of the people who stand at it.
 */

export const FURNITURE = {
  /** A table with the plan of the project on it, for two. */
  planTable: {
    x: 0,
    y: 0,
    rows: [
      'TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT',
      'TTwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwTT',
      'TTwt$tttwttt$ttwttttwt$tttwtttwwTT',
      'TTwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwTT',
      'TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT',
      'oooooooooooooooooooooooooooooooooo',
      'OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO',
      '.O..............................O.',
      '.O..............................O.',
      '.O..............................O.',
    ],
  },
  /** A counter with a coffee machine and a cup. */
  coffee: {
    x: 0,
    y: 0,
    rows: [
      '.xxxxxx.............',
      '.xmmmmx.............',
      '.xxrxxx.......vuuv..',
      '.xx..xx.......wwwwv.',
      'TTTTTTTTTTTTTTTTTTTT',
      'TTTTTTTTTTTTTTTTTTTT',
      'oooooooooooooooooooo',
      'oOooooooOoooooooOooo',
      'oOooooooOoooooooOooo',
      'OOOOOOOOOOOOOOOOOOOO',
    ],
  },
  /** A plant in a pot. */
  plant: {
    x: 0,
    y: 0,
    rows: [
      '...a.a...',
      '.aAa.aAa.',
      'aAaaAaaAa',
      '.aAaaAaa.',
      '..aAaAa..',
      '...oOo...',
      '..ooooo..',
      '..OOOOO..',
    ],
  },
} as const satisfies Record<string, Part>;

export type FurnitureName = keyof typeof FURNITURE;

/** The steam over the coffee machine, which rises while somebody stands at it. */
export const STEAM: readonly Part[] = [
  { x: 3, y: -2, rows: ['f.', '.f'] },
  { x: 3, y: -2, rows: ['.f', 'f.'] },
];
