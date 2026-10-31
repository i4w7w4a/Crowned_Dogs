/**
 * Keeps downloaded game files (models, textures, sounds, the engine) in the browser's
 * Cache Storage, so a restart loads them from disk instead of the network. Each file is
 * keyed by a hash of its content taken at build time: a changed model is fetched again,
 * an unchanged one never is. The dev server has no hashes, so nothing is cached there.
 * Must be imported before anything that fetches.
 */
declare const __CD_ASSET_VERSIONS__: Record<string, string>;

const VERSIONS: Record<string, string> = typeof __CD_ASSET_VERSIONS__ === "object" ? __CD_ASSET_VERSIONS__ : {};
const CACHE = "cd-assets-v1";
const base = new URL(".", document.baseURI);

function versioned(input: RequestInfo | URL): string | null {
  const href = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
  const url = new URL(href, document.baseURI);
  if (url.origin !== base.origin || !url.pathname.startsWith(base.pathname)) return null;
  const path = decodeURIComponent(url.pathname.slice(base.pathname.length));
  const v = VERSIONS[path];
  return v ? `${url.origin}${url.pathname}?cdv=${v}` : null;
}

function install(): void {
  if (!Object.keys(VERSIONS).length || typeof caches === "undefined") return;
  const network = window.fetch.bind(window);
  let open: Promise<Cache | null> | null = null;
  const cache = () => (open ??= caches.open(CACHE).catch(() => null));
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const key = (init?.method ?? (input instanceof Request ? input.method : "GET")) === "GET" ? versioned(input) : null;
    const store = key ? await cache() : null;
    if (!key || !store) return network(input, init);
    const hit = await store.match(key).catch(() => undefined);
    if (hit) return hit;
    const response = await network(input, init);
    if (response.ok && response.status === 200) store.put(key, response.clone()).catch(() => {});
    return response;
  };
  // Old versions of changed files are dropped once the game has settled.
  setTimeout(async () => {
    const store = await cache();
    if (!store) return;
    const live = new Set(Object.keys(VERSIONS).map((path) => `${base.origin}${base.pathname}${encodeURI(path)}?cdv=${VERSIONS[path]}`));
    for (const request of await store.keys().catch(() => [] as readonly Request[])) {
      if (!live.has(request.url)) store.delete(request).catch(() => {});
    }
  }, 20_000);
}

// Ask the browser not to clear saved progress and cached files when space runs low.
// Browsers decide on their own; most grant it silently to pages the player keeps using.
try { navigator.storage?.persist?.().catch(() => {}); } catch { /* not offered here */ }
try { install(); } catch { /* the network still works */ }
