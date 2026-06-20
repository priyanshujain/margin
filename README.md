# margin

An offline desktop app for writing books and exporting them to print-ready PDF and reflowable EPUB — a focused, free alternative to InDesign for text-first books (novels, non-fiction).

## Stack

- **Tauri 2** (Rust shell) — offline by construction; the entire export pipeline runs locally, nothing hits the network.
- **React + TipTap** — the semantic writing surface.
- **Typst** (embedded as a Rust crate) — print-ready PDF with real book typography.
- **Built-in EPUB3** generator + zip packager.

One semantic document feeds three renderers — the editor, the PDF, and the EPUB — from a single design-token theme (**Quiet Press**: Literata for the page, Hanken Grotesk for the chrome). Both fonts are bundled.

## Develop

```sh
pnpm install
pnpm tauri dev     # the real desktop app: live PDF dock, file dialogs, PDF/EPUB export
pnpm dev           # browser-only UI preview (Tauri APIs are inert)
```

The first `tauri dev` build is slow because it compiles Typst.

## Using it

- A book saves as a single `.margin` file (JSON). Open / Save with ⌘O / ⌘S.
- Click the title in the top bar for **Book setup** (title, author, ISBN, trim size, language).
- Insert images from the toolbar and choose a placement (inline / full-width / full-page / float).
- Export to **PDF** or **EPUB** from the titlebar menu.

## Layout

- `src/editor/` — TipTap editor, extensions, figure block, floating toolbar.
- `src/components/` — app shell, preview dock, book-setup panel, pdf.js renderer.
- `src/export/` — `typst.ts` (book → Typst source), `epub.ts` (book → EPUB3 files), `exporters.ts`.
- `src/store/`, `src/model/` — document model and state.
- `src-tauri/src/` — `pdf.rs` (Typst compile), `epub.rs` (zip packager), `project.rs` (file IO).
