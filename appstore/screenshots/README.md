# Mac App Store screenshots

Five 2560x1600 frames for the Margin listing: `frame-1.png` through `frame-5.png`.

Each frame is an HTML page in `src/`, rendered at a 2560x1600 viewport with a device pixel ratio of
1. The app window inside the frame is not a bitmap capture. It is a CSS rebuild of the real app in
`src/app.css`, with rules and values lifted from `shared/css/tokens.css` and `src/styles/app.css`,
laid out at the app's own 1440x900 and scaled once into the 1600x1000 window well. That is why the
type is sharp: it is real text at render time, not a resampled screenshot.

| File | Frame | Shows |
| --- | --- | --- |
| `frame-1.png` | Write | The writing surface, preview dock closed |
| `frame-2.png` | Preview | The editor beside the live page preview |
| `frame-3.png` | Proof | Three proofing marks and an open spelling popover |
| `frame-4.png` | Publish | The PDF export preview panel over the editor |
| `frame-5.png` | Yours | The library, no sign-in anywhere |

## Re-rendering

Serve this directory and screenshot each page at a 2560x1600 viewport, dpr 1:

```
python3 -m http.server 8731
```

Load `http://127.0.0.1:8731/src/frame-1.html` and capture the viewport to `frame-1.png` here. Check
every output with `sips -g pixelWidth -g pixelHeight frame-*.png`; App Store Connect rejects
anything that is not exactly 2560x1600.

Type comes from Google Fonts (Literata for headlines and prose, Hanken Grotesk for the interface),
so the render needs a network connection.

## Editing

`src/frame.css` is the frame surface: the eyebrow, headline, supporting line and the window well.
Every length in it is a real output pixel.

`src/app.css` is the app window. Every length in it is an app pixel, so a value copied out of the
product's own stylesheet can go in unchanged. Keep it that way; the whole point is that the
screenshots stay honest about what the app looks like.

Copy lives in each `src/frame-N.html`, along with that frame's window markup, since every frame
shows a different state of the app.

## Sample content

The frames use "The Quiet Hours" by Elena Marsh, an invented author. The prose is original and
written for this listing, so it is safe to publish. If you change it, keep it general non-fiction
rather than novel writing: the listing positions Margin as a writing app, not as software for one
genre.
