# Errors and feedback

## Quiet errors, loud logs

The bar is Mailspring. The user runs it against the same Google account and has never seen a sync
error in it. What Mailspring actually does: retries connection errors at the call site, shows nothing
for a single failure of any kind, raises a visible error only after five exits in five minutes, and
logs every caught exception to a per-account file.

So: a transient failure is the status chip's business and the next poll's, never a toast. Toast only
what a person can act on, which is a short list: paused, signed out, a missing permission, a write
that was dropped for good.

Every failure goes to the app's log file in the app data directory, `margin-mail.log` and its
equivalents. Engine passes, message bodies, IPC errors through the `call` wrapper, uncaught webview
errors. When the user reports an error, read that file before theorising.

An app that toasts on the first failure and logs nothing to disk gives the user noise and gives you
nothing to debug with. That is what this replaced.

## No silent waits

Any action that waits on the network or a slow operation changes something on screen at once. A
control that looks identical before and after being pressed reads as broken, and a second press
fires the call twice.

The pattern, per the apps' own `docs/conventions.md`: a string phase union on the handler
(`"idle" | "fetching" | "error"`), `data-phase` or `data-busy` on the control, `disabled` while in
flight, a present-tense label ("Loading images...", "Sending"), and an outcome either way. Primitives
carry the busy styling themselves, so the Banner action takes `busy` and `busyLabel` and Confirm
relabels while busy.

Never leave a `.catch(() => {})` on a user-pressed action. Never ship a button whose command nothing
registers: a dead control is the limit case of the same complaint.

The user's words, after clicking "Show images" and watching nothing happen for several seconds:
"giving this feeling of stuck is extremely bad ux".
