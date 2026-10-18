//! The consent page in front of the app on a phone, so the process stays foreground and its
//! loopback listener keeps accepting. Chrome Custom Tab on Android, SFSafariViewController on iOS
//! for the loopback flow, and ASWebAuthenticationSession on iOS for the custom scheme flow.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::open_in_browser;

static WAITING: AtomicBool = AtomicBool::new(false);
static DISMISSED: AtomicBool = AtomicBool::new(false);

/// How an `authenticate` session ended.
#[derive(Debug)]
pub enum Finished {
    Redirect(url::Url),
    /// No callback URL: closed by the user (`None`) or never shown (`Some(reason)`).
    Abandoned(Option<String>),
}

pub fn open(app: &tauri::AppHandle, url: &str) {
    DISMISSED.store(false, Ordering::SeqCst);
    WAITING.store(true, Ordering::SeqCst);
    present(app, url);
}

pub fn close(app: &tauri::AppHandle) {
    WAITING.store(false, Ordering::SeqCst);
    dismiss(app);
}

/// For `await_code`'s `cancelled`.
pub fn cancelled() -> bool {
    DISMISSED.load(Ordering::SeqCst)
}

/// The user closed the consent page. Only meaningful while the listener is still waiting.
pub fn note_dismissed() {
    if WAITING.load(Ordering::SeqCst) {
        DISMISSED.store(true, Ordering::SeqCst);
    }
}

/// Android cannot observe the Custom Tab closing, so the app coming back to the foreground while
/// the listener still waits is taken as the dismissal.
#[cfg(target_os = "android")]
pub fn note_resumed() {
    note_dismissed();
}

#[cfg(target_os = "ios")]
pub fn authenticate(
    app: &tauri::AppHandle,
    url: &str,
    scheme: &str,
    done: fn(tauri::AppHandle, Finished),
) {
    ios::authenticate(app, url, scheme, done);
}

#[cfg(target_os = "ios")]
fn present(app: &tauri::AppHandle, url: &str) {
    ios::present(app, url);
}

#[cfg(target_os = "ios")]
fn dismiss(app: &tauri::AppHandle) {
    ios::dismiss(app);
}

#[cfg(target_os = "ios")]
mod ios {
    use block2::{DynBlock, RcBlock};
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, NSObject, Sel};
    use objc2::{ClassType, msg_send, sel};
    use objc2_foundation::{NSError, NSString, NSURL};
    use tauri::Manager;

    use std::cell::RefCell;

    use super::{Finished, open_in_browser};

    #[link(name = "SafariServices", kind = "framework")]
    unsafe extern "C" {}

    thread_local! {
        static PRESENTED: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
        static DELEGATE: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    }

    extern "C-unwind" fn did_finish(_this: &AnyObject, _cmd: Sel, _controller: *mut AnyObject) {
        PRESENTED.with(|slot| slot.borrow_mut().take());
        DELEGATE.with(|slot| slot.borrow_mut().take());
        super::note_dismissed();
    }

    fn delegate_class() -> &'static AnyClass {
        thread_local! {
            static CLASS: &'static AnyClass = {
                let mut builder = ClassBuilder::new(c"MarginConsentDelegate", NSObject::class())
                    .expect("MarginConsentDelegate is registered once and by nobody else");
                unsafe {
                    builder.add_method(
                        sel!(safariViewControllerDidFinish:),
                        did_finish as extern "C-unwind" fn(_, _, _),
                    );
                }
                builder.register()
            };
        }
        CLASS.with(|class| *class)
    }

    pub fn present(app: &tauri::AppHandle, url: &str) {
        let Some(window) = app.get_webview_window("main") else {
            return open_in_browser(app, url);
        };
        let inner_app = app.clone();
        let inner_url = url.to_string();
        let posted = window.with_webview(move |webview| {
            let presented = unsafe {
                let view = webview.inner().cast::<AnyObject>();
                let ui_window: Option<Retained<AnyObject>> = msg_send![view, window];
                let root: Option<Retained<AnyObject>> = match &ui_window {
                    Some(ui_window) => msg_send![&**ui_window, rootViewController],
                    None => None,
                };
                match root {
                    Some(root) => show(&root, &inner_url),
                    None => false,
                }
            };
            if !presented {
                open_in_browser(&inner_app, &inner_url);
            }
        });
        if posted.is_err() {
            open_in_browser(app, url);
        }
    }

    unsafe fn show(root: &AnyObject, url: &str) -> bool {
        let Some(class) = AnyClass::get(c"SFSafariViewController") else {
            return false;
        };
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else {
            return false;
        };
        unsafe {
            let allocated: Allocated<AnyObject> = msg_send![class, alloc];
            let controller: Option<Retained<AnyObject>> = msg_send![allocated, initWithURL: &*url];
            let Some(controller) = controller else {
                return false;
            };

            let delegate: Retained<AnyObject> = msg_send![delegate_class(), new];
            let () = msg_send![&*controller, setDelegate: &*delegate];
            DELEGATE.with(|slot| *slot.borrow_mut() = Some(delegate));

            let completion: Option<&DynBlock<dyn Fn()>> = None;
            let () = msg_send![root, presentViewController: &*controller, animated: true, completion: completion];
            PRESENTED.with(|slot| *slot.borrow_mut() = Some(controller));
        }
        true
    }

    pub fn dismiss(app: &tauri::AppHandle) {
        let _ = app.run_on_main_thread(|| {
            let Some(controller) = PRESENTED.with(|slot| slot.borrow_mut().take()) else {
                return;
            };
            DELEGATE.with(|slot| slot.borrow_mut().take());
            unsafe {
                let completion: Option<&DynBlock<dyn Fn()>> = None;
                let () = msg_send![&*controller, dismissViewControllerAnimated: true, completion: completion];
            }
        });
    }

    #[link(name = "AuthenticationServices", kind = "framework")]
    unsafe extern "C" {}

    /// `ASWebAuthenticationSessionErrorCodeCanceledLogin`.
    const CANCELED_LOGIN: isize = 1;

    thread_local! {
        static SESSION: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
        static ANCHOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
        static ANCHOR_WINDOW: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    }

    extern "C-unwind" fn presentation_anchor(
        _this: &AnyObject,
        _cmd: Sel,
        _session: *mut AnyObject,
    ) -> *mut AnyObject {
        ANCHOR_WINDOW.with(|slot| match slot.borrow().as_ref() {
            Some(window) => Retained::as_ptr(window).cast_mut(),
            None => std::ptr::null_mut(),
        })
    }

    fn anchor_class() -> &'static AnyClass {
        thread_local! {
            static CLASS: &'static AnyClass = {
                let mut builder = ClassBuilder::new(c"MarginConsentAnchor", NSObject::class())
                    .expect("MarginConsentAnchor is registered once and by nobody else");
                unsafe {
                    builder.add_method(
                        sel!(presentationAnchorForWebAuthenticationSession:),
                        presentation_anchor as extern "C-unwind" fn(_, _, _) -> _,
                    );
                }
                builder.register()
            };
        }
        CLASS.with(|class| *class)
    }

    pub fn authenticate(
        app: &tauri::AppHandle,
        url: &str,
        scheme: &str,
        done: fn(tauri::AppHandle, Finished),
    ) {
        let Some(window) = app.get_webview_window("main") else {
            return open_in_browser(app, url);
        };
        let inner_app = app.clone();
        let inner_url = url.to_string();
        let scheme = scheme.to_string();
        let posted = window.with_webview(move |webview| {
            let started = unsafe {
                let view = webview.inner().cast::<AnyObject>();
                let ui_window: Option<Retained<AnyObject>> = msg_send![view, window];
                match ui_window {
                    Some(ui_window) => start(&inner_app, ui_window, &inner_url, &scheme, done),
                    None => false,
                }
            };
            if !started {
                open_in_browser(&inner_app, &inner_url);
            }
        });
        if posted.is_err() {
            open_in_browser(app, url);
        }
    }

    unsafe fn start(
        app: &tauri::AppHandle,
        ui_window: Retained<AnyObject>,
        url: &str,
        scheme: &str,
        done: fn(tauri::AppHandle, Finished),
    ) -> bool {
        let Some(class) = AnyClass::get(c"ASWebAuthenticationSession") else {
            return false;
        };
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else {
            return false;
        };
        let scheme = NSString::from_str(scheme);

        let handler_app = app.clone();
        let handler = RcBlock::new(move |callback: *mut NSURL, error: *mut NSError| {
            finished(&handler_app, callback, error, done);
        });

        unsafe {
            let allocated: Allocated<AnyObject> = msg_send![class, alloc];
            let session: Option<Retained<AnyObject>> = msg_send![
                allocated,
                initWithURL: &*url,
                callbackURLScheme: &*scheme,
                completionHandler: &*handler,
            ];
            let Some(session) = session else {
                return false;
            };

            let () = msg_send![&*session, setPrefersEphemeralWebBrowserSession: false];

            let anchor: Retained<AnyObject> = msg_send![anchor_class(), new];
            let () = msg_send![&*session, setPresentationContextProvider: &*anchor];
            ANCHOR_WINDOW.with(|slot| *slot.borrow_mut() = Some(ui_window));
            ANCHOR.with(|slot| *slot.borrow_mut() = Some(anchor));

            let started: bool = msg_send![&*session, start];
            SESSION.with(|slot| *slot.borrow_mut() = Some(session));
            started
        }
    }

    fn finished(
        app: &tauri::AppHandle,
        callback: *mut NSURL,
        error: *mut NSError,
        done: fn(tauri::AppHandle, Finished),
    ) {
        if !callback.is_null() {
            let absolute = unsafe {
                let string: Retained<NSString> = msg_send![callback, absoluteString];
                string.to_string()
            };
            if let Ok(parsed) = url::Url::parse(&absolute) {
                done(app.clone(), Finished::Redirect(parsed));
            }
            return;
        }

        let code = if error.is_null() {
            CANCELED_LOGIN
        } else {
            unsafe { msg_send![error, code] }
        };
        let reason = if code == CANCELED_LOGIN {
            None
        } else {
            Some(format!(
                "The sign-in sheet could not be shown (error {code})."
            ))
        };
        done(app.clone(), Finished::Abandoned(reason));
    }
}

#[cfg(target_os = "android")]
fn present(app: &tauri::AppHandle, url: &str) {
    let literal = match serde_json::to_string(url) {
        Ok(literal) => literal,
        Err(_) => return open_in_browser(app, url),
    };
    if eval(app, &format!("window.__androidAuthTab?.open({literal})")).is_err() {
        open_in_browser(app, url);
    }
}

#[cfg(target_os = "android")]
fn dismiss(app: &tauri::AppHandle) {
    let _ = eval(app, "window.__androidAuthTab?.close()");
}

#[cfg(target_os = "android")]
fn eval(app: &tauri::AppHandle, script: &str) -> Result<(), String> {
    use tauri::Manager;

    let window = app
        .get_webview_window("main")
        .ok_or("there is no window to run the bridge from")?;
    window.eval(script).map_err(|e| e.to_string())
}
