use objstore_s3::{Config, S3Store, authorization, signing_key};

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

#[tokio::test]
async fn conformance_against_local_minio() {
    let endpoint = env("OBJSTORE_S3_ENDPOINT", "http://127.0.0.1:9100");
    let host = endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');
    if std::net::TcpStream::connect(host).is_err() {
        eprintln!("skipped: no S3 server at {endpoint}, start tools/devtest/objstore-servers.sh");
        return;
    }
    let mut config = Config::new(
        endpoint,
        env("OBJSTORE_S3_REGION", "us-east-1"),
        env("OBJSTORE_S3_BUCKET", "objstore-test"),
        env("OBJSTORE_S3_ACCESS_KEY", "objstore"),
        env("OBJSTORE_S3_SECRET", "objstore-secret"),
    );
    config.allow_http = true;
    config.page_size = Some(7);
    let store = S3Store::new(config).unwrap();
    objstore_conformance::run(&store, &objstore_conformance::unique_root("s3")).await;
}

#[test]
fn refuses_plain_http_unless_allowed() {
    let config = Config::new("http://example.com", "auto", "b", "k", "s");
    assert!(S3Store::new(config.clone()).is_err());
    assert!(
        S3Store::new(Config {
            allow_http: true,
            ..config
        })
        .is_ok()
    );
    assert!(S3Store::new(Config::new("https://example.com", "auto", "a/b", "k", "s")).is_err());
}

#[test]
fn signing_key_matches_published_derivation() {
    let key = signing_key(
        "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        "20120215",
        "us-east-1",
        "iam",
    );
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "f4780e2d9f65fa895f9c67b32ce1baf0b0d8a43505a000a1a9e090d414db404d"
    );
}

#[test]
fn authorization_matches_get_vanilla() {
    let header = authorization(
        "AKIDEXAMPLE",
        "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        "us-east-1",
        "service",
        "GET",
        "/",
        "",
        &[
            ("host", "example.amazonaws.com".to_string()),
            ("x-amz-date", "20150830T123600Z".to_string()),
        ],
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "20150830T123600Z",
    );
    assert_eq!(
        header,
        "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
         SignedHeaders=host;x-amz-date, \
         Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
    );
}
