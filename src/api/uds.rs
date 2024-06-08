use anyhow::{anyhow, bail};
use base64::Engine;
use serde::Serialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Serialize, Debug)]
pub struct RenderRequest<'a> {
    left_image: Option<&'a [u8]>,
    right_image: Option<&'a [u8]>,
}

#[derive(Serialize, Debug)]
struct RenderRequestInner {
    #[serde(skip_serializing_if = "Option::is_none")]
    left_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right_image: Option<String>,
}

impl<'a> TryFrom<RenderRequest<'a>> for RenderRequestInner {
    type Error = anyhow::Error;

    fn try_from(value: RenderRequest) -> Result<Self, Self::Error> {
        if value.left_image.is_none() && value.right_image.is_none() {
            bail!("At least one image must be provided");
        }

        let encoder = base64::engine::general_purpose::STANDARD;
        let left_image = value.left_image.map(|buf| encoder.encode(buf));
        let right_image = value.right_image.map(|buf| encoder.encode(buf));
        Ok(Self {
            left_image,
            right_image,
        })
    }
}

pub struct UdsClient {
    path: PathBuf,
}

impl UdsClient {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        info!(?path, "Connecting to UDS socket");
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn send_request(&mut self, request: RenderRequest) -> anyhow::Result<String> {
        let mut stream = UnixStream::connect(self.path.as_path())?;
        
        let request = RenderRequestInner::try_from(request)?;
        let req_json = serde_json::to_string(&request)?;

        let http_request = format!(
            "POST /render/base64 HTTP/1.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            req_json.len(),
            req_json
        );
        stream.write_all(http_request.as_bytes())?;

        let start = std::time::Instant::now();
        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        println!("{:?}", start.elapsed());
        

        let body_start = response
            .find("\r\n\r\n")
            .ok_or(anyhow!("Invalid HTTP response"))?
            + 4;
        let body = &response[body_start..];
        
        Ok(body.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::renderer::Renderer;
    #[test]
    fn test_make_unix_socket_request() {
        let mut renderer = Renderer::new();
        renderer.render_cpu(10, 10, &[100, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        let left_image = renderer.save_to_in_memory_png().unwrap();

        let mut uds = UdsClient::new("/tmp/led-matrix.sock").unwrap();
        let request = RenderRequest {
            left_image: Some(&left_image),
            right_image: None,
        };
        assert!(uds.send_request(request).is_ok());
    }
}
