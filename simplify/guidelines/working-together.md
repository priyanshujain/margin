# Working together

## Finish the task

Do not call a task done until it is complete and tested. Do not hand back a green test suite and
ask the user to try it themselves.

For any app with a `justfile`, the last step is `just install`. That recipe quits the running app,
builds the bundle with `cargo build --bins --features tauri/custom-protocol --release`, replaces the
copy in `/Applications` and reopens it. The user tests on the installed app, so a fix that exists
only in `target/` is a fix they cannot see.

Never leave a second build running alongside it. A plain `cargo build --release` is a different
feature set, so the two rebuild every Tauri-dependent crate separately and fight over the target
lock. Kill the earlier build first.

## A bug is yours the moment you see it

Do not dismiss a bug as pre-existing. It does not matter that it was there before the change. When
you see a bug, fix it, or say plainly that you are leaving it and why.

## Never start the dev server

Do not run `pnpm tauri dev` or `pnpm dev`. The user keeps their own instance running and HMR picks
up edits in it. Vite is on `strictPort: 1420`, so a second dev server fails with `ELIFECYCLE` and
can kill the user's app. In Margin Mail the dev instance also shares the real app data directory,
so a dev run against real mail is a live-data accident waiting to happen.

Before any step that needs the running app, check for one:

    ps aux | grep -E "margin-app|vite"

If one is running, use it. If none is, ask. Do not start one.

For front-end work that a browser can exercise, a scratch server on a spare port is the safe route:
`npx vite --port 5199 --strictPort` in the background, driven with the Playwright tools, killed when
done. It never touches 1420. The caveat is that `isDesktop` (`"__TAURI_INTERNALS__" in window`) is
false there, so desktop-gated UI is absent unless you stub the invoke boundary. Each app's `src/dev`
directory already does exactly that; use it rather than inventing another stub.

## Fan out subagents for batches, but orient first

When the user hands over a batch of unrelated bugs, or asks for "an army of subagents", run them in
parallel. The rules that make it work:

Orient yourself first. Have the root causes in hand before spawning anything, or you get four agents
guessing in parallel instead of one person thinking.

One agent per bug, each with an explicit list of files it may edit. Shared files like `lib.rs` and
`mockIpc.ts` are edited with Edit and never with Write, or agents silently overwrite each other.

Tell every agent it may not run `pnpm tauri dev` or `just install`.

Integrate the work yourself, run the whole gate once, then install once at the end.

## Research the real products before designing

When asked for the best UX, or told to "dig through" other apps, research the actual products on the
internet: their docs, support pages, changelogs, what their users complain about. Searching only the
repo and designing from memory is not research, and the user has said so in those words.

Do repo orientation in parallel with the web research, not instead of it. Cite what you found when
you present the design, so the user can check it.

The reference products this suite is measured against: Mailspring, Superhuman, HEY, Thunderbird and
Apple Mail for mail; whatever the equivalent is for the app in hand. Name the one you compared to.

## Be blunt

Lead with the problem. If something the user said or assumed is wrong, say so and say why.
Disagreeing is the useful thing. Do not manufacture agreement to end a disagreement and do not fold
the moment they push back. If they reaffirm after hearing the argument, note the disagreement and do
it their way.

When you are unsure, say you are unsure. Vague hedging that reads as agreement is worse than "I do
not know".
