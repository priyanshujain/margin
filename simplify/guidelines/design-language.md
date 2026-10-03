# Design language

One hand made all four apps and they should look like it. Warm paper palette, Hanken Grotesk for UI,
Literata for headings, `data-theme` light and dark, hand-written CSS on a shared token set, no CSS
framework and no component library.

The tokens, the six bundled faces and the title bar glyph paths are the parts the apps must agree on
and they live in `margin-shared`. A glyph is a design decision, not a detail: when each app drew its
own search icon they drifted, and the result was two products from the same hand that did not look
related.

## Keyboard first

Every app is driven from the keyboard. Single keys, not chords, following the vocabulary the user
already knows from the products in that category. Margin Mail takes Gmail's and Superhuman's single
keys plus HEY's verbs.

Navigation steps by the smallest useful unit. In Margin Calendar the arrows and `h`/`l` move one day,
never a whole week, because a week jump destroys positional memory and positional memory is most of
the speed of a keyboard-driven app. The user's words for the week jump were that they "absolutely
hate" it.

## No third-party marks

No provider logos anywhere. No Google button. If a flow needs to know which provider a user is on,
infer it rather than asking them to pick a brand.

## Ask for the identifier, not the category

The add-account flow is address first. One email field, and the app reads the domain and routes:
Gmail domains go to browser sign-in with a `login_hint`, Microsoft hosts get an honest "not here
yet", everything else gets a sign-in step that names the servers it discovered before asking for a
password. This is the shape Thunderbird's Account Hub, Spark and the new Outlook use.

A fork that asks people to classify their own mailbox makes them answer a question before they know
what the answers cost. A big Google button beside a small "other" button reads as a Gmail client.
The user rejected that layout twice.

## A permission gate is a state, not an error message

When the OS gates a feature, the section shows one plain line for the state and one button that
fixes it, either asking for permission or opening the system's own pane. Every control that depends
on the permission is disabled until the answer is yes. State first, controls second.

What this is not: an error sentence sitting under live controls, a second button beside the first, or
copy that blames a named operating system. That is a puzzle rather than a state, and it is what
every other app already gets right.

## Restraint

No AI features unless the design was drawn with them in it. No settings for decisions the app should
make. Two font slots, body and heading, because a document that lets its author pick a face per
paragraph is a word processor, and none of these is one. Scale, leading and measure belong to the
stylesheet, which decided them once for every document.
