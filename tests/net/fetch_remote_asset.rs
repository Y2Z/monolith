#[cfg(test)]
mod failing {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use monolith::core::MonolithOptions;
    use monolith::net::{Credentials, fetch_remote_asset};
    use monolith::url::Url;
    use reqwest::blocking::Client;

    #[test]
    fn responses_that_break_off_are_not_assets() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 4096]);
            // Promises more than it ends up sending
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\nshort");
        });
        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };

        let result = fetch_remote_asset(
            &Client::new(),
            None,
            &Credentials::default(),
            &options,
            &url,
            &url,
            None,
        );

        assert!(result.is_err());
    }
}
