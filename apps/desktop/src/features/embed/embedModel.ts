/**
 * What the embed view works out without drawing anything: whether an address
 * may be kept, which form of it is made for showing inside another page, how
 * a long address is shortened, and how the view's content is read and
 * written.
 */

/** What the view keeps of its own. */
export interface EmbedContent {
  /** The address that is shown, always https. Nothing while none is chosen. */
  address: string | null;
}

/** The answer to "may this address be kept?": the address in its clean form, or why not. */
export type AddressCheck = { ok: true; address: string } | { ok: false; reason: string };

/** The one sentence a refused address is answered with. */
export const ONLY_HTTPS = 'Only addresses that begin with https:// can be shown.';

/**
 * Checks an address typed or pasted by the manager. It must parse, be
 * https, and have a host. Spaces around it are dropped; anything else is
 * refused in a plain sentence. The address that comes back is the one to
 * keep.
 */
export function checkAddress(typed: string): AddressCheck {
  const trimmed = typed.trim();
  if (trimmed === '') {
    return { ok: false, reason: 'Paste an address first.' };
  }
  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return { ok: false, reason: ONLY_HTTPS };
  }
  if (url.protocol !== 'https:') {
    return { ok: false, reason: ONLY_HTTPS };
  }
  if (url.hostname === '') {
    return { ok: false, reason: 'That address has no site in it.' };
  }
  // A name with credentials in it would be kept in the content and sent on
  // every write; nothing here needs them.
  if (url.username !== '' || url.password !== '') {
    return { ok: false, reason: 'An address with a name or password in it cannot be shown.' };
  }
  return { ok: true, address: url.href };
}

/** Whether a host is `name` or one of its subdomains. */
function isHostOf(host: string, name: string): boolean {
  return host === name || host.endsWith(`.${name}`);
}

/** A path as its non-empty parts. */
function partsOf(url: URL): string[] {
  return url.pathname.split('/').filter((part) => part !== '');
}

/** The seconds of a YouTube `t` or `start` value such as `90`, `90s` or `1m30s`. */
function secondsOf(value: string | null): number | null {
  if (value === null) {
    return null;
  }
  const plain = /^(\d+)s?$/.exec(value);
  if (plain?.[1] !== undefined) {
    return Number(plain[1]);
  }
  const parts = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/.exec(value);
  if (parts === null || value === '') {
    return null;
  }
  return Number(parts[1] ?? 0) * 3600 + Number(parts[2] ?? 0) * 60 + Number(parts[3] ?? 0);
}

/** An identifier in an address of a video site: letters, digits, dashes and underscores. */
function isIdentifier(part: string | undefined): part is string {
  return part !== undefined && /^[\w-]+$/.test(part);
}

function youtube(url: URL, host: string): string | null {
  const parts = partsOf(url);
  let id: string | undefined;
  if (host === 'youtu.be') {
    id = parts[0];
  } else if (parts[0] === 'watch') {
    id = url.searchParams.get('v') ?? undefined;
  } else if (parts[0] === 'shorts' || parts[0] === 'embed' || parts[0] === 'live') {
    id = parts[1];
  }
  if (!isIdentifier(id)) {
    return null;
  }
  const start = secondsOf(url.searchParams.get('t') ?? url.searchParams.get('start'));
  const query = start === null ? '' : `?start=${start}`;
  return `https://www.youtube-nocookie.com/embed/${id}${query}`;
}

function vimeo(url: URL, host: string): string | null {
  const parts = partsOf(url);
  if (host === 'player.vimeo.com') {
    return null;
  }
  const id = parts.find((part) => /^\d+$/.test(part));
  if (id === undefined) {
    return null;
  }
  // An unlisted video has a second number, the hash, after its own.
  const after = parts[parts.indexOf(id) + 1];
  const hash = after !== undefined && /^[0-9a-f]+$/i.test(after) ? `?h=${after}` : '';
  return `https://player.vimeo.com/video/${id}${hash}`;
}

function loom(url: URL): string | null {
  const parts = partsOf(url);
  if (parts[0] !== 'share' || !isIdentifier(parts[1])) {
    return null;
  }
  return `https://www.loom.com/embed/${parts[1]}`;
}

const FIGMA_KINDS = ['file', 'design', 'board', 'proto', 'slides', 'deck'];

function figma(url: URL): string | null {
  const parts = partsOf(url);
  if (parts[0] === undefined || !FIGMA_KINDS.includes(parts[0]) || parts[1] === undefined) {
    return null;
  }
  return `https://www.figma.com/embed?embed_host=teamwright&url=${encodeURIComponent(url.href)}`;
}

/** A Google file's address: `/<document|spreadsheets|presentation>/d/<id>` or `/d/e/<key>` when published. */
function google(url: URL): string | null {
  const parts = partsOf(url);
  const [kind, d, first, second] = parts;
  if (d !== 'd' || first === undefined) {
    return null;
  }
  const base = `https://docs.google.com/${kind}/d`;
  if (first === 'e' && isIdentifier(second)) {
    // Published to the web.
    switch (kind) {
      case 'document':
        return `${base}/e/${second}/pub?embedded=true`;
      case 'spreadsheets':
        return `${base}/e/${second}/pubhtml?widget=true&headers=false`;
      case 'presentation':
        return `${base}/e/${second}/embed`;
      default:
        return null;
    }
  }
  if (!isIdentifier(first)) {
    return null;
  }
  // Shared with the person who looks: the preview of the file.
  switch (kind) {
    case 'document':
    case 'spreadsheets':
      return `${base}/${first}/preview`;
    case 'presentation':
      return `${base}/${first}/embed`;
    default:
      return null;
  }
}

/**
 * The form of an address that is made for showing inside another page.
 * Well-known kinds are turned: a YouTube watch address or a youtu.be
 * address (on the no-cookie host), a Vimeo address, a Loom share, a Figma
 * file or design, and a Google Docs, Sheets or Slides address that is
 * published or shared. An address the function does not know comes back as
 * it was.
 */
export function embedAddress(address: string): string {
  let url: URL;
  try {
    url = new URL(address);
  } catch {
    return address;
  }
  if (url.protocol !== 'https:') {
    return address;
  }
  const host = url.hostname.toLowerCase();
  let turned: string | null = null;
  if (
    host === 'youtu.be' ||
    isHostOf(host, 'youtube.com') ||
    isHostOf(host, 'youtube-nocookie.com')
  ) {
    turned = youtube(url, host);
  } else if (isHostOf(host, 'vimeo.com')) {
    turned = vimeo(url, host);
  } else if (host === 'loom.com' || host === 'www.loom.com') {
    turned = loom(url);
  } else if (host === 'figma.com' || host === 'www.figma.com') {
    turned = figma(url);
  } else if (host === 'docs.google.com') {
    turned = google(url);
  }
  return turned ?? address;
}

/**
 * An address short enough for the bar: the middle is replaced by an
 * ellipsis when it is longer than `limit` characters, so that both the
 * site and the end of the path stay readable. The scheme is dropped.
 */
export function shortenAddress(address: string, limit = 56): string {
  const bare = address.replace(/^https:\/\//, '').replace(/\/$/, '');
  if (bare.length <= limit || limit < 8) {
    return bare;
  }
  const keep = limit - 1;
  const head = Math.ceil(keep * 0.6);
  const tail = keep - head;
  return `${bare.slice(0, head)}…${bare.slice(bare.length - tail)}`;
}

/** Reads what the view keeps. Content that is missing, damaged or from another version is an empty view. */
export function parseContent(content: string | null): EmbedContent {
  const empty: EmbedContent = { address: null };
  if (content === null || content.trim() === '') {
    return empty;
  }
  try {
    const parsed: unknown = JSON.parse(content);
    if (typeof parsed !== 'object' || parsed === null) {
      return empty;
    }
    const address = (parsed as { address?: unknown }).address;
    if (typeof address !== 'string') {
      return empty;
    }
    // Content from elsewhere is checked as an address typed would be.
    const checked = checkAddress(address);
    return checked.ok ? { address: checked.address } : empty;
  } catch {
    return empty;
  }
}

/** The text a view's content is written as. */
export function serializeContent(content: EmbedContent): string {
  return JSON.stringify({ address: content.address });
}
