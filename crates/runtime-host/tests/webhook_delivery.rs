use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use vrcx_0_application_core::WebClient;
use vrcx_0_persistence::storage::StorageService;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_runtime_host::notification::send_json_webhook_with_retry;

struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "vrcx-0-webhook-delivery-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn test_web(name: &str) -> (TestDir, WebClient) {
    let dir = TestDir::new(name);
    let db = Arc::new(DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap());
    let storage = StorageService::new(&dir.0.join("storage.json")).unwrap();
    let web = WebClient::new(
        &storage,
        db.as_ref(),
        "http://localhost:9000".into(),
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap();
    (dir, web)
}

#[tokio::test]
async fn webhook_redirects_are_failures_and_are_not_retried() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/webhook", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        serve_responses(
            listener,
            &["HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"],
        )
    });
    let (_dir, web) = test_web("redirect");

    let failure = send_json_webhook_with_retry(&web, &url, json!({"event": "test"}))
        .await
        .unwrap_err();

    assert_eq!(failure.status, Some(302));
    assert_eq!(failure.attempts, 1);
    assert_eq!(server.join().unwrap(), 1);
}

#[tokio::test]
async fn webhook_rate_limit_retry_uses_the_same_sender_and_reports_attempts() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/webhook", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        serve_responses(
            listener,
            &[
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"retry_after\":0}",
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ],
        )
    });
    let (_dir, web) = test_web("rate-limit");

    let outcome = send_json_webhook_with_retry(&web, &url, json!({"event": "test"}))
        .await
        .unwrap();

    assert_eq!(outcome.status, 204);
    assert_eq!(outcome.attempts, 2);
    assert_eq!(server.join().unwrap(), 2);
}

fn serve_responses(listener: TcpListener, responses: &[&str]) -> usize {
    let mut count = 0;
    for response in responses {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request);
        stream.write_all(response.as_bytes()).unwrap();
        count += 1;
    }
    count
}
