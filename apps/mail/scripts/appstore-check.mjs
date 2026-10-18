#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const metadata = join(root, "appstore", "metadata");
const submission = process.argv.includes("--submission");
const errors = [];
const pending = [];
const read = (name) => {
  const path = join(metadata, name);
  return existsSync(path) ? readFileSync(path, "utf8").trim() : "";
};

for (const [name, limit, bytes] of [
  ["en-US/name.txt", 30],
  ["en-US/subtitle.txt", 30],
  ["en-US/keywords.txt", 100, true],
  ["en-US/promotional_text.txt", 170],
  ["en-US/description.txt", 4000],
  ["en-US/release_notes.txt", 4000],
  ["en-US/beta_description.txt", 4000],
  ["review_notes.txt", 4000, true],
]) {
  const value = read(name);
  const length = bytes ? Buffer.byteLength(value) : [...value].length;
  if (!value) errors.push(`missing metadata: ${name}`);
  if (length > limit) errors.push(`${name}: ${length} exceeds ${limit} ${bytes ? "bytes" : "characters"}`);
  if (name.endsWith("name.txt") && length < 2) errors.push(`${name}: must contain at least two characters`);
}

for (const name of ["support_url", "marketing_url", "privacy_url"]) {
  try {
    const url = new URL(read(`en-US/${name}.txt`));
    if (!["https:", "http:"].includes(url.protocol) || !url.hostname || url.username || url.password) {
      throw new Error("invalid public URL");
    }
  } catch {
    errors.push(`en-US/${name}.txt: must be an absolute public HTTP or HTTPS URL`);
  }
}

for (const name of ["primary_category", "copyright"]) {
  if (!read(`${name}.txt`)) errors.push(`missing metadata: ${name}.txt`);
}
for (const name of ["review_first_name", "review_last_name", "review_email", "review_phone", "beta_feedback_email"]) {
  const value = read(`${name}.txt`);
  if (!value) pending.push(`missing contact: ${name}.txt`);
  else if (name.includes("email") && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) errors.push(`${name}.txt: invalid email`);
  else if (name === "review_phone" && !/^\+[1-9][\d ()-]{6,}$/.test(value)) errors.push(`${name}.txt: use international format starting with +`);
}

const screenshots = join(root, "appstore", "screenshots");
const files = existsSync(screenshots) ? readdirSync(screenshots).filter((name) => name.endsWith(".png")) : [];
if (files.length === 0) pending.push("missing Mac App Store screenshots");
if (files.length > 10) errors.push("Mac App Store supports at most 10 screenshots");
const sizes = new Set(["1280x800", "1440x900", "2560x1600", "2880x1800"]);
for (const name of files) {
  const png = readFileSync(join(screenshots, name));
  if (png.length < 33 || png.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a" || png.toString("ascii", 12, 16) !== "IHDR") {
    errors.push(`${name}: invalid PNG header`);
    continue;
  }
  const size = `${png.readUInt32BE(16)}x${png.readUInt32BE(20)}`;
  if (!sizes.has(size)) errors.push(`${name}: unsupported Mac screenshot size ${size}`);
}

pending.push("working review mailbox and credentials must be supplied privately in App Store Connect");
pending.push("App Privacy, age rating, content rights and export compliance require owner review in App Store Connect");
pending.push("public support and privacy pages must cover Margin Mail's mailbox and backup behavior");
pending.push("signed sandboxed build and real mailbox review flow require manual verification");

for (const error of errors) console.error(`error: ${error}`);
for (const item of pending) console.log(`pending: ${item}`);
console.log(errors.length ? "appstore: draft validation failed" : "appstore: local draft metadata valid");
if (submission && pending.length) console.error("appstore: submission readiness is blocked by the pending items above");
process.exitCode = errors.length || (submission && pending.length) ? 1 : 0;
