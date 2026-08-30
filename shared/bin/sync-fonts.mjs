#!/usr/bin/env node
// Copies this package's font files into an app's public/fonts, or checks that they already match.
//
// The copies exist because both apps' `src-tauri/src/pdf.rs` reads the same files with
// `include_bytes!`, so the bytes have to be on disk at a path cargo can see before any npm install
// has run. Serving them out of node_modules would mean a cargo build that fails on a fresh clone
// until the front end was installed, which is a worse trade than a vendored copy with a named
// upstream and a check that fails loudly when the two drift.
//
//   node bin/sync-fonts.mjs <app-dir>            copy, reporting what changed
//   node bin/sync-fonts.mjs <app-dir> --check    compare only, exit 1 on any difference

import { readdirSync, readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const source = resolve(here, "..", "fonts");

const args = process.argv.slice(2);
const check = args.includes("--check");
const target = args.find((a) => !a.startsWith("--"));

if (!target) {
  console.error("usage: sync-fonts.mjs <app-dir> [--check]");
  process.exit(2);
}

const dest = resolve(process.cwd(), target, "public", "fonts");
const files = readdirSync(source).filter((f) => f.endsWith(".ttf") || f.endsWith(".txt")).sort();

if (!check) mkdirSync(dest, { recursive: true });

const wrong = [];
let copied = 0;

for (const file of files) {
  const from = join(source, file);
  const to = join(dest, file);
  const want = readFileSync(from);
  const have = existsSync(to) ? readFileSync(to) : null;

  if (have !== null && have.equals(want)) continue;

  if (check) {
    wrong.push(`${have === null ? "missing" : "differs"}: ${file}`);
    continue;
  }
  writeFileSync(to, want);
  copied += 1;
}

// A file the app has and the package does not is reported rather than deleted. It is far more
// likely to be a face somebody is in the middle of adding than something to throw away, and this
// script does not get to be the reason an asset disappears.
if (existsSync(dest)) {
  for (const file of readdirSync(dest)) {
    if (!files.includes(file)) wrong.push(`not in the package, left alone: ${file}`);
  }
}

if (check) {
  if (wrong.length === 0) {
    console.log(`fonts match margin-shared (${files.length} files)`);
    process.exit(0);
  }
  console.error("public/fonts is out of step with margin-shared:");
  for (const line of wrong) console.error(`  ${line}`);
  console.error("\nrun: pnpm fonts:sync");
  process.exit(1);
}

console.log(copied === 0 ? `fonts already current (${files.length} files)` : `synced ${copied} file(s)`);
for (const line of wrong) console.log(`  note: ${line}`);
