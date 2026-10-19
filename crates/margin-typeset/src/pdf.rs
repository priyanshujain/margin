// PDF export: Typst source in, PDF bytes out, compiled in this process.
//
// Nothing about this touches the user's markdown. The converter that produces the source lives in
// src/export/typst.ts and reads the ProseMirror document; this module compiles what it is given
// and writes the result where a native save panel pointed. A document that has never been saved
// exports exactly as well as one that has.
//
// The compiler sees no filesystem and no network. Everything it can open is put in front of it by
// hand: the nine bundled faces, the images this call was handed, and a vendored copy of mitex
// served at `/mitex/`. That is the whole world, so a document cannot make the exporter fetch a
// package, and the export works on a machine that has never been online.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use typst::diag::{Severity, SourceDiagnostic, Warned};
use typst::layout::PagedDocument;
use typst_as_lib::TypstEngine;

use crate::{Image, Warning};

// The faces a document is set in, one file per weight and slope.
//
// Not the variable fonts in packages/core/fonts/ that the editor itself renders with, and that is not
// duplication for its own sake: Typst does not support a variable axis, warns that it does not, and
// lays the text out at the default instance regardless of the weight asked for. Every heading, every
// bold run and every callout label would come out at 400, which is a PDF where the hierarchy the
// author can see on screen is gone. These nine are static instances cut from those same four files;
// fonts/PROVENANCE.md is how, and is what to repeat when a face is updated.
static FACES: [&[u8]; 9] = [
    include_bytes!("../fonts/Literata-Regular.ttf"),
    include_bytes!("../fonts/Literata-SemiBold.ttf"),
    include_bytes!("../fonts/Literata-Bold.ttf"),
    include_bytes!("../fonts/Literata-Italic.ttf"),
    include_bytes!("../fonts/Literata-BoldItalic.ttf"),
    include_bytes!("../fonts/HankenGrotesk-Regular.ttf"),
    include_bytes!("../fonts/HankenGrotesk-Bold.ttf"),
    include_bytes!("../fonts/HankenGrotesk-Italic.ttf"),
    include_bytes!("../fonts/HankenGrotesk-BoldItalic.ttf"),
];

// The other four families the editor offers, in the variable files packages/core/fonts already
// ships for the editor, and loaded only when a document is actually set in one.
//
// These carry the caveat the nine above exist to avoid, and it is not worth cutting eight more
// static instances to fix. Typst lays a variable file out at its default instance, so a document
// set in EB Garamond exports with its headings at the same weight as its body: the face is right,
// the hierarchy is flatter than the screen. The two the app defaults to are the two that export
// perfectly, and choosing one of these is choosing a face rather than a fallback.
static EB_GARAMOND: [&[u8]; 2] = [
    include_bytes!("../../../packages/core/fonts/EBGaramond-VF.ttf"),
    include_bytes!("../../../packages/core/fonts/EBGaramond-Italic-VF.ttf"),
];
static LORA: [&[u8]; 2] = [
    include_bytes!("../../../packages/core/fonts/Lora-VF.ttf"),
    include_bytes!("../../../packages/core/fonts/Lora-Italic-VF.ttf"),
];
static SOURCE_SERIF: [&[u8]; 2] = [
    include_bytes!("../../../packages/core/fonts/SourceSerif4-VF.ttf"),
    include_bytes!("../../../packages/core/fonts/SourceSerif4-Italic-VF.ttf"),
];
static FRAUNCES: [&[u8]; 2] = [
    include_bytes!("../../../packages/core/fonts/Fraunces-VF.ttf"),
    include_bytes!("../../../packages/core/fonts/Fraunces-Italic-VF.ttf"),
];

/// The bytes behind one `BundledFont` id from src/model/fonts.ts.
///
/// Literata and Hanken Grotesk are absent on purpose rather than missing: `FACES` already holds
/// their nine static instances and they are loaded for every export, so naming either here would
/// hand Typst the same family twice and let the variable copy win.
fn bundled_faces(id: &str) -> &'static [&'static [u8]] {
    match id {
        "eb-garamond" => &EB_GARAMOND,
        "lora" => &LORA,
        "source-serif" => &SOURCE_SERIF,
        "fraunces" => &FRAUNCES,
        _ => &[],
    }
}

// mitex 0.2.5, vendored under vendor/mitex with its LICENSE, and served as ordinary
// paths under `/mitex/` rather than through Typst's package system. A package spec would mean a
// download on first export and a cache directory to keep, for a dependency that is 380K and never
// changes. Every file in the package has to be here: lib.typ imports mitex.typ relatively, that
// imports specs/mod.typ, and mitex.typ loads the wasm module beside it.
static MITEX_LIB: &str = include_str!("../vendor/mitex/lib.typ");
static MITEX_MAIN: &str = include_str!("../vendor/mitex/mitex.typ");
static MITEX_SPECS: &str = include_str!("../vendor/mitex/specs/mod.typ");
static MITEX_PRELUDE: &str = include_str!("../vendor/mitex/specs/prelude.typ");
static MITEX_LATEX: &str = include_str!("../vendor/mitex/specs/latex/standard.typ");
static MITEX_WASM: &[u8] = include_bytes!("../vendor/mitex/mitex.wasm");

/// The import path the generated source uses, and the contract with src/export/typst.ts:
/// `#import "/mitex/lib.typ": mitex, mi`.
const MITEX_ROOT: &str = "/mitex/";

/// What `/mitex/lib.typ` answers with on a second attempt, after a formula stopped the first one.
///
/// It keeps the names the generated source imports and sets every formula as the LaTeX the user
/// wrote, which is the readable thing to do with a formula nothing can typeset. Nothing here loads
/// the wasm module, because that is what failed.
static MITEX_PLAIN_LIB: &str = r#"#let mitex-source(it) = {
  if type(it) == str { it } else if type(it) == content and it.has("text") { it.text } else { repr(it) }
}
#let mi(it, ..args) = raw(mitex-source(it))
#let mimath(it, ..args) = raw(mitex-source(it))
#let mitext(it) = raw(mitex-source(it))
#let mitex(it, mode: "math", ..args) = block(raw(mitex-source(it)))
#let mitex-convert(it, mode: "math", spec: none) = mitex-source(it)
"#;

/// A 1x1 transparent image in each format Typst picks from a file extension, so that an image the
/// exporter could not read leaves a gap on the page rather than killing the export.
///
/// One per format because Typst trusts the extension over the bytes: handing PNG bytes to
/// `#image("photo.jpg")` is a decode error, which is the hard failure this exists to avoid. The
/// formats Typst does not recognise from an extension fall through to sniffing the data, so PNG is
/// the right default for those.
const PLACEHOLDER_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4//8/AwAI/AL+p5qgoAAAAABJRU5ErkJggg==";
const PLACEHOLDER_JPEG: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAP//////////////////////////////////////////////////////////////////////////////////////wAALCAABAAEBAREA/8QAFAABAAAAAAAAAAAAAAAAAAAAA//EABQQAQAAAAAAAAAAAAAAAAAAAAD/2gAIAQEAAD8AR//Z";
const PLACEHOLDER_GIF: &str = "R0lGODdhAQABAIEAAP///wAAAAAAAAAAACwAAAAAAQABAAAIBAABBAQAOw==";
const PLACEHOLDER_WEBP: &str = "UklGRkAAAABXRUJQVlA4WAoAAAAQAAAAAAAAAAAAQUxQSAIAAAAAAFZQOCAYAAAAMAEAnQEqAQABAAFAJiWkAANwAP789AAA";
const PLACEHOLDER_SVG: &str =
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>";

fn placeholder_for(path: &str) -> Vec<u8> {
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let encoded = match extension.as_str() {
        "jpg" | "jpeg" => PLACEHOLDER_JPEG,
        "gif" => PLACEHOLDER_GIF,
        "webp" => PLACEHOLDER_WEBP,
        "svg" | "svgz" => return PLACEHOLDER_SVG.as_bytes().to_vec(),
        _ => PLACEHOLDER_PNG,
    };
    STANDARD.decode(encoded).unwrap_or_default()
}

fn font_list(families: &[String], last_resort: &str) -> String {
    let mut names: Vec<String> = families.iter().map(|f| format!("\"{f}\"")).collect();
    names.push(format!("\"{last_resort}\""));
    format!("({})", names.join(", "))
}

/// The only Typst this module writes, and it names font families and nothing else.
///
/// It belongs here rather than in the converter because which faces the compiler can see is this
/// module's business: nine bundled, plus whichever monospace and math families the machine turned
/// out to have. src/export/typst.ts names none of them for exactly that reason.
///
/// Math is the one that cannot be left alone. Typst sets every equation to the single family
/// "New Computer Modern Math" and switches font fallback off while it does it, so on a machine
/// without that family one formula is a hard compile error rather than a warning. Naming a list is
/// what makes a formula typeset at all, and Literata closes both lists so that a machine with none
/// of the candidates gets a page that reads badly and a warning saying so, instead of no PDF.
///
/// Only families that are actually installed are named, because Typst warns once for every family
/// it was asked for and could not find, and a hopeful list would end every export with a toast
/// about fonts nobody chose.
fn font_preamble(fallbacks: &crate::fonts::Fallbacks) -> String {
    format!(
        "#set text(font: \"Literata\")\n\
         #show raw: set text(font: {})\n\
         #show math.equation: set text(font: {})\n",
        font_list(&fallbacks.monospace, "Literata"),
        font_list(&fallbacks.math, "Literata"),
    )
}

/// Adds a warning, or bumps the count of one already there.
///
/// Forty formulas that would not typeset are one problem and not forty toasts, and the same
/// message arriving twice is the overwhelmingly common case: one broken construct repeated down a
/// document.
fn note(warnings: &mut Vec<Warning>, kind: &str, message: String) {
    if let Some(existing) = warnings
        .iter_mut()
        .find(|w| w.kind == kind && w.message == message)
    {
        existing.count += 1;
        return;
    }
    warnings.push(Warning {
        kind: kind.to_string(),
        message,
        count: 1,
    });
}

fn in_mitex(span: typst::syntax::Span) -> bool {
    span.id().is_some_and(|id| {
        id.vpath()
            .as_rooted_path()
            .to_string_lossy()
            .starts_with(MITEX_ROOT)
    })
}

fn touches_mitex(diagnostic: &SourceDiagnostic) -> bool {
    in_mitex(diagnostic.span) || diagnostic.trace.iter().any(|point| in_mitex(point.span))
}

/// Which kind a diagnostic gets, or `None` for one the user should never see.
///
/// A warning raised inside the vendored mitex sources with nothing in its trace leading back out
/// of them is mitex talking about itself: a deprecation in a pinned copy of a dependency, the same
/// on every machine and in every document, and nothing anybody reading a toast can act on. One
/// with the document in its trace is a formula that did not come out right, which is a "math"
/// warning and is worth saying.
fn kind_for(diagnostic: &SourceDiagnostic) -> Option<&'static str> {
    if !in_mitex(diagnostic.span) {
        return Some("typst");
    }
    if diagnostic.trace.iter().any(|point| !in_mitex(point.span)) {
        return Some("math");
    }
    None
}

/// One pass over the compiler. `lib` is what `/mitex/lib.typ` answers with, which is the whole
/// difference between a formula that typesets and one that is shown as the source the user wrote.
///
/// The one error in this crate that is not a `String`, and deliberately. The caller has to ask
/// whether the failure came from inside mitex before it decides whether a second attempt is worth
/// making, and that question is asked of the diagnostics' spans. Formatting them first would throw
/// away the only thing the answer depends on.
fn compile_once(
    source: &str,
    lib: &str,
    binaries: &[(&str, Vec<u8>)],
    fonts: &[Vec<u8>],
) -> Result<(Vec<u8>, Vec<SourceDiagnostic>), Vec<SourceDiagnostic>> {
    let engine = TypstEngine::builder()
        .main_file(source)
        .fonts(fonts.iter().map(|font| font.as_slice()))
        .with_static_source_file_resolver([
            ("/mitex/lib.typ", lib),
            ("/mitex/mitex.typ", MITEX_MAIN),
            ("/mitex/specs/mod.typ", MITEX_SPECS),
            ("/mitex/specs/prelude.typ", MITEX_PRELUDE),
            ("/mitex/specs/latex/standard.typ", MITEX_LATEX),
        ])
        .with_static_file_resolver(
            binaries
                .iter()
                .map(|(path, bytes)| (*path, bytes.as_slice())),
        )
        .build();

    let Warned { output, warnings } = engine.compile();
    let document: PagedDocument = output.map_err(|e| match e {
        typst_as_lib::TypstAsLibError::TypstSource(diagnostics) => {
            diagnostics.into_iter().collect()
        }
        other => vec![SourceDiagnostic::error(
            typst::syntax::Span::detached(),
            other.to_string(),
        )],
    })?;
    let bytes = typst_pdf::pdf(&document, &Default::default())
        .map_err(|d| d.into_iter().collect::<Vec<_>>())?;
    Ok((bytes, warnings.into_iter().collect()))
}

pub fn compile(
    source: String,
    images: Vec<Image>,
    bundled: &[String],
    system: &[String],
) -> Result<(Vec<u8>, Vec<Warning>), String> {
    let mut warnings: Vec<Warning> = Vec::new();

    let mut binaries: Vec<(&str, Vec<u8>)> = Vec::with_capacity(images.len() + 1);
    for image in &images {
        match &image.bytes {
            Ok(bytes) => binaries.push((image.path.as_str(), bytes.clone())),
            Err(e) => {
                note(&mut warnings, "image", e.clone());
                binaries.push((image.path.as_str(), placeholder_for(&image.path)));
            }
        }
    }
    binaries.push(("/mitex/mitex.wasm", MITEX_WASM.to_vec()));

    // The two families this app does not bundle, a monospace for code blocks and a math face for
    // formulas, come off the system. Neither is shown in the editor, both are large, and shipping a
    // mono nobody ever sees on screen so that a code block matches across machines is a trade this
    // project has decided against.
    let mut fallbacks = crate::fonts::fallbacks();
    let mut fonts: Vec<Vec<u8>> = FACES.iter().map(|face| face.to_vec()).collect();
    fonts.append(&mut fallbacks.fonts);

    // The face the author chose, if it is not one of the two already in `FACES`. A system family is
    // read off this machine, which is the one place an export can differ between two computers, and
    // is the caveat the picker states next to the "System" group.
    for id in bundled {
        fonts.extend(bundled_faces(id).iter().map(|face| face.to_vec()));
    }
    for family in system {
        fonts.extend(crate::fonts::system_faces(family));
    }

    let source = format!("{}{source}", font_preamble(&fallbacks));

    let failure = match compile_once(&source, MITEX_LIB, &binaries, &fonts) {
        Ok((bytes, diagnostics)) => {
            for diagnostic in &diagnostics {
                if let Some(kind) = kind_for(diagnostic) {
                    note(&mut warnings, kind, diagnostic.message.to_string());
                }
            }
            return Ok((bytes, warnings));
        }
        Err(failure) => failure,
    };

    // mitex turns LaTeX into Typst by running a wasm module over it, and a formula it cannot parse
    // is a hard error rather than a bad-looking equation. One `$\frac{$` a user typed halfway down
    // a page of notes would otherwise cost them the entire export, so the second attempt serves a
    // stand-in `/mitex/lib.typ` that sets every formula as the source the user actually wrote. The
    // page reads worse and the warning says so, which is a trade the user can act on.
    if !failure.iter().any(touches_mitex) {
        return Err(format_diagnostics(&failure));
    }
    let (bytes, diagnostics) = compile_once(&source, MITEX_PLAIN_LIB, &binaries, &fonts)
        // The first failure is the one worth reading: the second is whatever the stand-in tripped
        // over on the way, and the formula that started it is named in the first.
        .map_err(|_| format_diagnostics(&failure))?;

    note(
        &mut warnings,
        "math",
        "a formula could not be typeset, so every formula is shown as the source it was written in"
            .to_string(),
    );
    for diagnostic in &diagnostics {
        if let Some(kind) = kind_for(diagnostic) {
            note(&mut warnings, kind, diagnostic.message.to_string());
        }
    }
    Ok((bytes, warnings))
}

fn format_diagnostics(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let kind = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let mut message = format!("{kind}: {}", diagnostic.message);
            for hint in &diagnostic.hints {
                message.push_str(&format!("\n  hint: {hint}"));
            }
            message
        })
        .collect::<Vec<_>>()
        .join("\n")
}
