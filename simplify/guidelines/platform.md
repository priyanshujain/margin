# Platform

Facts that cost a day each to find and are not derivable from the repo or the crate docs.

## macOS notifications need UN and a bundle signature

Verified on 2026-09-05 with throwaway Swift probes. On macOS 26, `NSUserNotificationCenter`, which is
what `tauri-plugin-notification`, `notify-rust` and `mac-notification-sys` all post through, reports
successful delivery and shows nothing. It never registers the app in Notification Center and never
prompts. The failure is completely silent: the plugin returns `Ok` and the system log says nothing.

`UNUserNotificationCenter` prompts and shows, but only when the process is a real `NSApplication` in
a bundle that carries a bundle signature. Ad-hoc is enough, `codesign -s -` works. The linker
signature a plain `tauri build` leaves behind is not, and produces "Notifications are not allowed
for this application".

That same error text also appears when the user dismissed the permission banner, so the message
alone does not tell you which of the two happened.

A banner that reads "Margin Mail" over the single word "Notification" is not the app posting that
word. It is the system hiding the content, and on this machine the cause was the global Show
previews setting being Never, at the bottom of System Settings > Notifications, which every app on
"Default" inherits. To read that without prompting anybody, build a throwaway bundle that only calls
`getNotificationSettings` and writes `showPreviewsSetting.rawValue` to a file: 0 always, 1 when
unlocked, 2 never. Do not trust `content_visibility` in `com.apple.ncprefs`, which read 1 while the
API said never.

Margin Mail posts through `src-tauri/src/notify/macos.rs` and its `just build` sources the signing
env file.

One correction to what was previously written down here: the other three apps do not have this bug,
because they do not post notifications at all. Checked on 2026-09-06, neither
`tauri-plugin-notification` nor `@tauri-apps/plugin-notification` appears in Margin's, Margin
Calendar's or Margin Docs' manifests, and Margin Calendar's `notify` at `App.tsx:27` is a toast
helper with nothing to do with the system. The point still stands for the day one of them wants
notifications, because the plugin is what anyone would reach for and it silently does nothing.

## Reading the installed app's state

A Tauri app's `localStorage` lives under
`~/Library/WebKit/<bundle-id>/WebsiteData/Default/*/*/LocalStorage/localstorage.sqlite3`. Copy that
file and its `-wal` sibling to `/tmp` first, then read it with
`sqlite3 ... "select key, hex(value) from ItemTable"`. Values are UTF-16LE.

The dev server origin has a separate store under `~/Library/WebKit/<package-name>/`, so a dev run
does not reproduce what the installed app shows. When a screenshot of the installed app disagrees
with what the code should draw, read the real state before theorising. Reading is fine. Never edit
that file.

## Mobile

Margin has an initialised iOS target (`src-tauri/gen/apple`, not gitignored) that builds and runs in
the simulator; typst, harper and reqwest all cross-compile. Android is not set up and needs the SDK,
NDK and JDK 17.

`isDesktop` in `src/ipc.ts` really means "is Tauri", so it is true on mobile. Anything genuinely
desktop-only has to be gated on something else.

WKWebView CSS problems that do not reproduce in a desktop browser, all fixed in Margin and worth
copying into any app that goes mobile: text auto-inflation of wide blocks needs
`html { -webkit-text-size-adjust: 100% }`; zoom on input focus needs `maximum-scale=1.0,
user-scalable=no` in the viewport; the keyboard scrolling the whole page and pushing the title bar
under the notch needs `body { position: fixed; inset: 0; overflow: hidden }` with the scrolling
confined to one pane; a caret drawn below its text needs a line-height at or above the face's natural
metrics, 1.4 rather than 1.16 for Literata.

Programmatic `.focus()` on iOS does not show the caret or keyboard without a real gesture, so those
states cannot be verified with `simctl` screenshots. A human has to tap.

Mobile OAuth cannot use a loopback listener. That is why Margin Calendar and Margin Mail both
register `tauri-plugin-deep-link` and take the answer back through a custom URI scheme, on desktop
too, so both flows are one code path with one difference in it.

## Responsive layout

The compact breakpoint is `(max-width: 899px)`, read through `useCompact()` in `src/useMedia.ts`. The
root element carries `data-compact` and the pane state. Desktop collapses the sidebar to width zero
and lets the main pane reclaim it. Compact turns the side panes into fixed slide-in drawers over a
full-width main pane, one at a time, behind a scrim, with extra title bar actions folded into an
overflow menu. Safe-area insets and a `--titlebar-h` token handle the notch.
