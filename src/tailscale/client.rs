use crate::platform::SocketPath;
use crate::tailscale::types::Status;
use base64::Engine;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::error::Error;
use std::fmt;

#[cfg(unix)]
use hyperlocal::{UnixConnector, Uri};

#[cfg(windows)]
use hyper_named_pipe::{NAMED_PIPE_SCHEME, NamedPipeConnector};

#[derive(Debug)]
pub enum TailscaleError {
    SocketConnection(String),
    HttpRequest(reqwest::Error),
    JsonParse(serde_json::Error),
    ApiError(String),
}

impl fmt::Display for TailscaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TailscaleError::SocketConnection(msg) => write!(f, "Socket connection error: {}", msg),
            TailscaleError::HttpRequest(err) => write!(f, "HTTP request error: {}", err),
            TailscaleError::JsonParse(err) => write!(f, "JSON parse error: {}", err),
            TailscaleError::ApiError(msg) => write!(f, "Tailscale API error: {}", msg),
        }
    }
}

impl Error for TailscaleError {}

impl From<reqwest::Error> for TailscaleError {
    fn from(err: reqwest::Error) -> Self {
        TailscaleError::HttpRequest(err)
    }
}

impl From<serde_json::Error> for TailscaleError {
    fn from(err: serde_json::Error) -> Self {
        TailscaleError::JsonParse(err)
    }
}

pub enum TailscaleClient {
    #[cfg(unix)]
    Unix {
        socket_path: String,
        client: Client<UnixConnector, Full<Bytes>>,
    },
    #[cfg(windows)]
    NamedPipe {
        pipe_path: String,
        client: Client<NamedPipeConnector, Full<Bytes>>,
    },
    Tcp {
        base_url: String,
        token: Option<String>,
        client: Client<HttpConnector, Full<Bytes>>,
    },
}

impl TailscaleClient {
    pub fn new() -> Result<Self, TailscaleError> {
        let socket_path = SocketPath::default_socket_path()
            .map_err(|e| TailscaleError::SocketConnection(e.to_string()))?;
        
        Self::from_socket_path(socket_path)
    }

    pub fn with_socket_path(socket_path: String) -> Result<Self, TailscaleError> {
        Self::from_socket_path(socket_path)
    }
    
    fn from_socket_path(socket_path: String) -> Result<Self, TailscaleError> {
        if socket_path.starts_with("tcp://") {
            let connector = HttpConnector::new();
            let client = Client::builder(TokioExecutor::new()).build(connector);

            // Parse tcp://host:port:token format
            let parts: Vec<&str> = socket_path
                .strip_prefix("tcp://")
                .unwrap_or(&socket_path)
                .split(':')
                .collect();
            let (base_url, token) = if parts.len() >= 3 {
                (
                    format!("http://{}:{}", parts[0], parts[1]),
                    Some(parts[2].to_string()),
                )
            } else {
                (
                    socket_path
                        .strip_prefix("tcp://")
                        .map(|s| format!("http://{}", s))
                        .unwrap_or(socket_path),
                    None,
                )
            };

            Ok(TailscaleClient::Tcp {
                base_url,
                token,
                client,
            })
        } else {
            #[cfg(unix)]
            {
                let connector = UnixConnector;
                let client = Client::builder(TokioExecutor::new()).build(connector);

                Ok(TailscaleClient::Unix {
                    socket_path,
                    client,
                })
            }
            #[cfg(windows)]
            {
                // Windows Named Pipe path
                let connector = NamedPipeConnector;
                let client = Client::builder(TokioExecutor::new()).build(connector);

                Ok(TailscaleClient::NamedPipe {
                    pipe_path: socket_path,
                    client,
                })
            }
            #[cfg(not(any(unix, windows)))]
            {
                Err(TailscaleError::SocketConnection(
                    "Platform not supported".to_string(),
                ))
            }
        }
    }

    pub async fn get_status(&self) -> Result<Status, TailscaleError> {
        self.get_status_with_peers(true).await
    }

    pub async fn get_status_without_peers(&self) -> Result<Status, TailscaleError> {
        self.get_status_with_peers(false).await
    }

    async fn get_status_with_peers(&self, include_peers: bool) -> Result<Status, TailscaleError> {
        let path = if include_peers {
            "/localapi/v0/status"
        } else {
            "/localapi/v0/status?peers=false"
        };

        let response = match self {
            #[cfg(unix)]
            TailscaleClient::Unix {
                socket_path,
                client,
            } => {
                let uri = Uri::new(socket_path, path);
                let request = self.build_request(uri, None)?;
                client.request(request).await.map_err(|e| {
                    TailscaleError::SocketConnection(format!("Failed to send request: {}", e))
                })?
            }
            #[cfg(windows)]
            TailscaleClient::NamedPipe { pipe_path, client } => {
                // Hex encode the pipe path for hyper-named-pipe
                let hex_encoded_pipe = hex::encode(pipe_path.as_bytes());
                let uri: hyper::Uri =
                    format!("{}://{}{}", NAMED_PIPE_SCHEME, hex_encoded_pipe, path)
                        .parse()
                        .map_err(|e| {
                            TailscaleError::SocketConnection(format!("Invalid URI: {}", e))
                        })?;
                let request = self.build_request(uri, None)?;
                client.request(request).await.map_err(|e| {
                    TailscaleError::SocketConnection(format!("Failed to send request: {}", e))
                })?
            }
            TailscaleClient::Tcp {
                base_url,
                token,
                client,
            } => {
                let uri: hyper::Uri = format!("{}{}", base_url, path)
                    .parse()
                    .map_err(|e| TailscaleError::SocketConnection(format!("Invalid URI: {}", e)))?;
                let request = self.build_request(uri, token.as_deref())?;
                client.request(request).await.map_err(|e| {
                    TailscaleError::SocketConnection(format!("Failed to send request: {}", e))
                })?
            }
        };

        self.handle_response(response).await
    }
    
    fn build_request(&self, uri: impl Into<hyper::Uri>, token: Option<&str>) -> Result<hyper::Request<Full<Bytes>>, TailscaleError> {
        let mut request_builder = hyper::Request::builder()
            .method(hyper::Method::GET)
            .uri(uri.into())
            .header("Host", "local-tailscaled.sock");

        // Add token authentication if available
        if let Some(token) = token {
            let auth_value = format!(":{}", token);
            let encoded = base64::engine::general_purpose::STANDARD.encode(auth_value);
            request_builder = request_builder.header("Authorization", format!("Basic {}", encoded));
        }

        request_builder
            .body(Full::new(Bytes::new()))
            .map_err(|e| TailscaleError::SocketConnection(format!("Failed to build request: {}", e)))
    }

    async fn handle_response(
        &self,
        response: hyper::Response<hyper::body::Incoming>,
    ) -> Result<Status, TailscaleError> {
        let status_code = response.status();
        if !status_code.is_success() {
            return Err(TailscaleError::ApiError(format!(
                "HTTP {}: {}",
                status_code,
                status_code.canonical_reason().unwrap_or("Unknown")
            )));
        }

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| {
                TailscaleError::SocketConnection(format!("Failed to read response body: {}", e))
            })?
            .to_bytes();

        let status: Status = serde_json::from_slice(&body_bytes).map_err(|e| {
            tracing::error!("Failed to parse Tailscale status JSON: {}", e);
            TailscaleError::JsonParse(e)
        })?;
        Ok(status)
    }

    pub async fn test_connection(&self) -> Result<(), TailscaleError> {
        self.get_status_without_peers().await.map(|_| ())
    }
}
