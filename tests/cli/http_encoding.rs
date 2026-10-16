use assert_cmd::cargo_bin_cmd;
use encoding_rs::SHIFT_JIS;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn archive(content_type: &str, body: Vec<u8>) -> Vec<u8> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let content_type = content_type.to_string();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
        stream.write_all(&body).unwrap();
    });
    let output = cargo_bin_cmd!(env!("CARGO_PKG_NAME"))
        .args(["-M", "-i", "-q", "-t", "10", &url])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn http_charset_overrides_conflicting_meta() {
    let html = "<meta http-equiv=content-type content=\"text/html;charset=Shift_JIS\"><title>テスト</title><p>こんにちは</p>";
    let output = String::from_utf8(archive(
        "text/html; charset=UTF-8",
        html.as_bytes().to_vec(),
    ))
    .unwrap();
    assert!(output.contains("<title>テスト</title>"));
    assert!(output.contains("<p>こんにちは</p>"));
    assert!(output.contains("text/html;charset=UTF-8"));
}

#[test]
fn http_without_valid_charset_uses_meta() {
    let html = "<meta charset=Shift_JIS><title>テスト</title>";
    let (body, _, errors) = SHIFT_JIS.encode(html);
    assert!(!errors);
    for content_type in ["text/html", "text/html; charset=unknown"] {
        let output = archive(content_type, body.to_vec());
        let (decoded, _, errors) = SHIFT_JIS.decode(&output);
        assert!(!errors);
        assert!(decoded.contains("<title>テスト</title>"));
    }
}

#[test]
fn bom_overrides_http_charset() {
    let mut body = vec![0xef, 0xbb, 0xbf];
    body.extend_from_slice("<meta charset=Shift_JIS><title>テスト</title>".as_bytes());
    let output = String::from_utf8(archive("text/html; charset=Shift_JIS", body)).unwrap();
    assert!(output.contains("<title>テスト</title>"));
    assert!(output.contains("charset=\"UTF-8\""));
}

#[test]
fn data_url_default_charset_does_not_override_meta() {
    let output = cargo_bin_cmd!(env!("CARGO_PKG_NAME"))
        .args([
            "-M",
            "-q",
            "data:text/html,%3Cmeta%20charset%3Dutf-8%3E%3Ctitle%3E%E3%83%86%E3%82%B9%E3%83%88%3C%2Ftitle%3E",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("<title>テスト</title>"));
    assert!(output.contains("charset=\"utf-8\""));
}
