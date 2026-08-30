# margin-shared

The faces, palette and glyphs [Margin](../) and Margin Docs both draw from. It is the one copy of
the decisions the two apps have to agree on: which six fonts they offer, what their named pairings
are, the 35 design tokens they share, and the title bar icon paths.

No build step and no dependencies. Both apps resolve the TypeScript source directly through their
bundler, so an edit here is live in both on the next dev server restart.

Margin consumes it as `"margin-shared": "file:./shared"`. Margin Docs, which lives in a separate
repository, uses a relative path to this directory; see its `package.json`.

The font binaries in `fonts/` are the source of truth, and each app keeps a vendored copy under
`public/fonts` because its Rust PDF exporter reads them with `include_bytes!` before any npm install
has run. Run `pnpm fonts:sync` in an app to refresh that copy and `pnpm fonts:check` to verify it.
