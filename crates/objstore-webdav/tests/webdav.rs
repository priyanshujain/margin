use objstore_webdav::{Config, WebDavStore};

#[tokio::test]
async fn conformance_against_local_server() {
    let url = std::env::var("OBJSTORE_WEBDAV_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9200/".to_string());
    let host = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap()
        .to_string();
    if std::net::TcpStream::connect(&host).is_err() {
        eprintln!("skipped: no WebDAV server at {url}, start tools/devtest/objstore-servers.sh");
        return;
    }
    let mut config = Config::new(url);
    config.username =
        Some(std::env::var("OBJSTORE_WEBDAV_USER").unwrap_or_else(|_| "objstore".to_string()));
    config.password = Some(
        std::env::var("OBJSTORE_WEBDAV_PASSWORD").unwrap_or_else(|_| "objstore-secret".to_string()),
    );
    config.allow_http = true;
    config.conditional_put = true;
    let store = WebDavStore::new(config.clone()).unwrap();
    objstore_conformance::run(&store, &objstore_conformance::unique_root("webdav")).await;

    config.conditional_put = false;
    let store = WebDavStore::new(config).unwrap();
    objstore_conformance::run(&store, &objstore_conformance::unique_root("webdav-plain")).await;
}
