use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Long-lived API traffic: a generous timeout for large batch responses, a short idle pool so a
    /// connection the machine slept through is not reused, and HTTP/2 pings to find dead ones.
    Api,
    /// Token endpoints: a shorter timeout and the same keep-alive, since a refresh lands once an
    /// hour on a connection nobody has checked since the last one.
    Auth,
}

pub fn client(profile: Profile) -> reqwest::Client {
    let builder = reqwest::Client::builder().connect_timeout(Duration::from_secs(10));
    let builder = match profile {
        Profile::Api => builder
            .timeout(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(30)),
        Profile::Auth => builder.timeout(Duration::from_secs(30)),
    };
    builder
        .http2_keep_alive_interval(Duration::from_secs(20))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        .tcp_keepalive(Duration::from_secs(30))
        .build()
        .expect("could not build the HTTP client")
}
