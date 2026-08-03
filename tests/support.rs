use unifi::UnifiConfig;

pub(super) fn test_config(url: impl Into<String>) -> UnifiConfig {
    UnifiConfig {
        url: url.into(),
        api_key: "test-key".into(),
        site: "default".into(),
        skip_tls_verify: true,
        legacy: false,
    }
}

pub(super) struct CaptureServer {
    addr: std::net::SocketAddr,
    handle: std::thread::JoinHandle<String>,
}

impl CaptureServer {
    pub(super) fn spawn(status: u16, body: &'static str) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind capture server");
        let addr = listener.local_addr().expect("capture server addr");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept one request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = std::io::Read::read(&mut stream, &mut buffer).expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if body_complete(&request) {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            std::io::Write::write_all(&mut stream, response.as_bytes()).expect("write response");
            String::from_utf8_lossy(&request).to_ascii_lowercase()
        });
        Self { addr, handle }
    }

    pub(super) fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub(super) fn request(self) -> String {
        self.handle.join().expect("capture thread should finish")
    }
}

pub(super) struct SequenceCaptureServer {
    addr: std::net::SocketAddr,
    handle: std::thread::JoinHandle<Vec<String>>,
}

impl SequenceCaptureServer {
    pub(super) fn spawn(responses: Vec<(u16, &'static str)>) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind sequence server");
        let addr = listener.local_addr().expect("sequence server addr");
        let handle = std::thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, body)| {
                    let (mut stream, _) = listener.accept().expect("accept request");
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let read =
                            std::io::Read::read(&mut stream, &mut buffer).expect("read request");
                        if read == 0 {
                            break;
                        }
                        request.extend_from_slice(&buffer[..read]);
                        if body_complete(&request) {
                            break;
                        }
                    }
                    let response = format!(
                        "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    std::io::Write::write_all(&mut stream, response.as_bytes())
                        .expect("write response");
                    String::from_utf8_lossy(&request).to_ascii_lowercase()
                })
                .collect()
        });
        Self { addr, handle }
    }

    pub(super) fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub(super) fn requests(self) -> Vec<String> {
        self.handle.join().expect("sequence thread should finish")
    }
}

fn body_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-length: "))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    request.len() >= header_end + 4 + content_length
}
