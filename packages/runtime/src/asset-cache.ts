/**
 * Keeps downloaded game files (models, textures, sounds, the engine) in the browser's
 * Cache Storage, so a restart loads them from disk instead of the network. Each file is
 * keyed by a hash of its content taken at build time: a changed model is fetched again,
 * an unchanged one never is. The dev server has no hashes, so nothing is cached there.
 * The runtime imports it first, before the engine itself is fetched.
 */
declare const __CD_ASSET_VERSIONS__: Record<string, string>;

const VERSIONS: Record<string, string> = typeof __CD_ASSET_VERSIONS__ === "object" ? __CD_ASSET_VERSIONS__ : {};
const CACHE = "cd-assets-v1";
const base = typeof document === "undefined" ? null : new URL(".", document.baseURI);

/** The build-relative path of a request for one of this page's files, or null. */
function local(base: URL, input: RequestInfo | URL): string | null {
  const href = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
  const url = new URL(href, document.baseURI);
  if (url.origin !== base.origin || !url.pathname.startsWith(base.pathname)) return null;
  return decodeURIComponent(url.pathname.slice(base.pathname.length));
}
const key = (base: URL, path: string) => `${base.origin}${base.pathname}${encodeURI(path)}?cdv=${VERSIONS[path]}`;

/** Whether the build ships this file next to the page (always no in dev, which has no list). */
export function shipped(input: string): boolean {
  if (!base) return false;
  const path = local(base, input);
  return path !== null && path in VERSIONS;
}

/**
 * The light copy of a big painted background (<name>.png -> <name>.lite.webp, built by vite.config.ts)
 * when the build ships one, else the same URL. The qualities that take it ask (litePictures in Rust).
 */
export function litePicture(url: string): string {
  if (!base || !/\.png([?#]|$)/i.test(url)) return url;
  const lite = url.replace(/\.png(?=[?#]|$)/i, ".lite.webp");
  return shipped(lite) ? lite : url;
}
if (typeof globalThis !== "undefined") (globalThis as { cdLitePicture?: (url: string) => string }).cdLitePicture = litePicture;

// The build ships a gzip copy (<file>.gz) of every big file that packs well (models, the engine,
// manifests). The game downloads that copy and unpacks it itself, so the saving does not depend on
// the host compressing. A host that already marks the copy as gzip hands over unpacked bytes, which
// pass through as they are.
const TYPES: Record<string, string> = { wasm: "application/wasm", json: "application/json", glb: "model/gltf-binary" };
async function unpacked(network: typeof fetch, path: string, init?: RequestInit): Promise<Response | null> {
  if (!base || !VERSIONS[`${path}.gz`] || typeof DecompressionStream === "undefined") return null;
  try {
    const response = await network(new URL(encodeURI(`${path}.gz`), base).href, init);
    if (!response.ok || !response.body) return null;
    const packed = new Uint8Array(await response.arrayBuffer());
    const body = packed[0] === 0x1f && packed[1] === 0x8b
      ? await new Response(new Blob([packed]).stream().pipeThrough(new DecompressionStream("gzip"))).arrayBuffer()
      : packed.buffer;
    const type = TYPES[path.split(".").pop()!] ?? "application/octet-stream";
    return new Response(body, { status: 200, headers: { "Content-Type": type, "Content-Length": String(body.byteLength) } });
  } catch {
    return null;
  }
}

function install(base: URL | null): void {
  if (!base || !Object.keys(VERSIONS).length) return;
  const network = window.fetch.bind(window);
  let open: Promise<Cache | null> | null = null;
  // Cache Storage exists only on https and localhost; a phone on the home network still gets the gzip copies.
  const cache = () => (open ??= typeof caches === "undefined" ? Promise.resolve(null) : caches.open(CACHE).catch(() => null));
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const get = (init?.method ?? (input instanceof Request ? input.method : "GET")) === "GET";
    const path = get ? local(base, input) : null;
    if (path === null || !VERSIONS[path]) return network(input, init);
    const store = await cache();
    const hit = store ? await store.match(key(base, path)).catch(() => undefined) : undefined;
    if (hit) return hit;
    const response = (await unpacked(network, path, init)) ?? (await network(input, init));
    if (store && response.ok && response.status === 200) store.put(key(base, path), response.clone()).catch(() => {});
    return response;
  };
  // Old versions of changed files are dropped once the game has settled.
  setTimeout(async () => {
    const store = await cache();
    if (!store) return;
    const live = new Set(Object.keys(VERSIONS).map((path) => key(base, path)));
    for (const request of await store.keys().catch(() => [] as readonly Request[])) {
      if (!live.has(request.url)) store.delete(request).catch(() => {});
    }
  }, 20_000);
}

// Ask the browser not to clear saved progress and cached files when space runs low.
// Browsers decide on their own; most grant it silently to pages the player keeps using.
if (base) {
  try { navigator.storage?.persist?.().catch(() => {}); } catch { /* not offered here */ }
  try { install(base); } catch { /* the network still works */ }
}
