// Typst source helpers shared by every app that exports through margin-typeset.

/**
 * A lone surrogate is half of a character, and it survives neither the JSON of the IPC boundary nor
 * UTF-8 on the other side of it. A paste out of a broken tool can carry one, and it must not be the
 * thing that decides whether an export happens.
 */
export function sanitize(value: string): string {
  let out = "";
  for (const ch of value) {
    const code = ch.codePointAt(0) ?? 0;
    out += code >= 0xd800 && code <= 0xdfff ? "�" : ch;
  }
  return out;
}

/**
 * The one place a string from a document becomes Typst source.
 *
 * A Typst string literal knows `\\`, `\"`, `\n`, `\r`, `\t` and `\u{...}` and nothing else, so a
 * control character is written as its escape and every other character stands for itself. There is
 * no construct inside a string literal for a `#`, a `$` or an `@` to open, so the escaping has one
 * rule instead of a list that goes out of date when Typst gains syntax.
 */
export function str(value: string): string {
  let out = '"';
  for (const ch of sanitize(value)) {
    const code = ch.codePointAt(0) ?? 0;
    if (ch === "\\") out += "\\\\";
    else if (ch === '"') out += '\\"';
    else if (ch === "\n") out += "\\n";
    else if (ch === "\r") out += "\\r";
    else if (ch === "\t") out += "\\t";
    else if (code < 0x20 || code === 0x7f) out += `\\u{${code.toString(16)}}`;
    else out += ch;
  }
  return `${out}"`;
}
