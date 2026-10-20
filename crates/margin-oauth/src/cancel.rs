use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The sign-ins an app has in flight, of which only the newest may finish. Starting another or
/// cancelling ends the one before, so a consent page answered after the user gave up adds nothing.
#[derive(Default)]
pub struct Attempts {
    current: Mutex<Option<Attempt>>,
}

/// One sign-in. Pass `|| attempt.is_cancelled()` to `await_code`, and check it again before storing
/// anything, because the code may have arrived just before the cancel did.
#[derive(Clone, Default)]
pub struct Attempt(Arc<AtomicBool>);

impl Attempt {
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl Attempts {
    pub fn begin(&self) -> Attempt {
        let attempt = Attempt::default();
        let mut slot = self.current.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(previous) = slot.replace(attempt.clone()) {
            previous.cancel();
        }
        attempt
    }

    pub fn cancel(&self) {
        let mut slot = self.current.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(attempt) = slot.take() {
            attempt.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::{CANCELLED, Pages, await_code, bind_loopback};

    #[test]
    fn a_new_attempt_or_a_cancel_ends_the_one_before() {
        let attempts = Attempts::default();
        let first = attempts.begin();
        let second = attempts.begin();
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
        attempts.cancel();
        assert!(second.is_cancelled());
        attempts.cancel();
    }

    #[test]
    fn a_cancelled_wait_stops_listening_so_a_late_redirect_finds_nobody() {
        let attempts = Attempts::default();
        let attempt = attempts.begin();
        let (listener, redirect) = bind_loopback().expect("a port");
        let port = redirect.rsplit(':').next().expect("a port").to_string();
        let waiting = attempt.clone();
        let wait = std::thread::spawn(move || {
            await_code(
                listener,
                "state",
                Instant::now() + Duration::from_secs(30),
                Pages {
                    app: "Test",
                    provider: "Test",
                },
                || waiting.is_cancelled(),
            )
        });
        std::thread::sleep(Duration::from_millis(200));
        attempts.cancel();
        let outcome = wait.join().expect("the wait ends");
        assert_eq!(outcome, Err(CANCELLED.to_string()));
        assert!(TcpStream::connect(format!("127.0.0.1:{port}")).is_err());
    }
}
