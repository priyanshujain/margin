//! Centres the macOS traffic lights on a title bar taller than the system's. AppKit has no inset
//! for this, so the button container is made taller and the buttons, measured from its bottom
//! edge, come down together. Nothing assumes a button size, which changed in macOS 26.
//! Reapplied on resize, focus and theme change, because AppKit lays the container out again.

#[cfg(target_os = "macos")]
use tauri::{Manager, Runtime, WebviewWindow, WindowEvent};

/// Centres the lights on a row `height` points tall now and keeps them centred. No-op off macOS.
#[cfg(target_os = "macos")]
pub fn install<R: Runtime>(window: &WebviewWindow<R>, height: f64) {
    apply(window, height);
    let app = window.app_handle().clone();
    let label = window.label().to_string();
    window.on_window_event(move |event| {
        if !matches!(
            event,
            WindowEvent::Resized(_) | WindowEvent::Focused(_) | WindowEvent::ThemeChanged(_)
        ) {
            return;
        }
        if let Some(window) = app.get_webview_window(&label) {
            apply(&window, height);
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn install<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>, _height: f64) {}

#[cfg(target_os = "macos")]
fn apply<R: Runtime>(window: &WebviewWindow<R>, height: f64) {
    let handle = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(ptr) = handle.ns_window() else { return };
        if ptr.is_null() {
            return;
        }
        mac::centre(unsafe { &*(ptr as *const objc2_app_kit::NSWindow) }, height);
    });
}

/// The drop needed to centre a button of `button_h` whose top sits `top` below the window's top,
/// or `None` when it is within half a point already.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn drop_for(height: f64, button_h: f64, top: f64) -> Option<f64> {
    const SLACK: f64 = 0.5;
    let drop = (height - button_h) / 2.0 - top;
    (drop.abs() >= SLACK).then_some(drop)
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2_app_kit::{NSWindow, NSWindowButton, NSWindowStyleMask};

    pub fn centre(window: &NSWindow, height: f64) {
        if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
            return;
        }
        let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
            return;
        };
        let Some(container) = (unsafe { close.superview().and_then(|view| view.superview()) })
        else {
            return;
        };
        let button = close.convertRect_toView(close.bounds(), None);
        let top = window.frame().size.height - (button.origin.y + button.size.height);
        let Some(drop) = super::drop_for(height, button.size.height, top) else {
            return;
        };
        let mut frame = container.frame();
        frame.size.height += drop;
        frame.origin.y -= drop;
        container.setFrame(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lights_drop_onto_the_middle_of_the_row_and_stay_put_once_there() {
        assert_eq!(drop_for(46.0, 14.0, 9.0), Some(7.0));
        assert_eq!(drop_for(46.0, 14.0, 16.0), None);
        assert_eq!(drop_for(46.0, 14.0, 16.3), None);
        assert_eq!(drop_for(46.0, 14.0, 20.0), Some(-4.0));
    }
}
