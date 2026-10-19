// Resolves the latest GitHub release to a direct download URL per OS, at build
// time. The picked file follows each platform's `match` order in site.ts.
// Falls back to the releases page if the release can't be read.

import { site, platforms } from "./site";

type Asset = { name: string; browser_download_url: string };

export type Release = {
  version: string;
  urls: Record<string, string>;
};

function pickAsset(assets: Asset[], match: string[]): string | null {
  for (const sfx of match) {
    const hit = assets.find((a) => a.name.toLowerCase().endsWith(sfx.toLowerCase()));
    if (hit) return hit.browser_download_url;
  }
  return null;
}

function fallback(): Release {
  return {
    version: "",
    urls: Object.fromEntries(platforms.map((p) => [p.id, site.releasesUrl])),
  };
}

async function fetchRelease(): Promise<Release> {
  try {
    const res = await fetch(`https://api.github.com/repos/${site.repo}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
      signal: AbortSignal.timeout(8000),
    });
    if (!res.ok) return fallback();
    const data = await res.json();
    const assets: Asset[] = data.assets || [];
    const urls = Object.fromEntries(
      platforms.map((p) => [p.id, pickAsset(assets, p.match) ?? site.releasesUrl]),
    );
    return { version: data.tag_name ?? "", urls };
  } catch {
    return fallback();
  }
}

let cached: Promise<Release> | null = null;

export function getRelease(): Promise<Release> {
  cached ??= fetchRelease();
  return cached;
}
