use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum PlatformError {
    UnsupportedOS(String),
    SocketNotFound(String),
    PermissionDenied(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::UnsupportedOS(os) => write!(f, "Unsupported operating system: {}", os),
            PlatformError::SocketNotFound(path) => {
                write!(f, "Tailscale socket not found at: {}", path)
            }
            PlatformError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl Error for PlatformError {}

pub struct SocketPath;

impl SocketPath {
    /// Get the default Tailscale socket path for the current platform
    pub fn default_socket_path() -> Result<String, PlatformError> {
        #[cfg(target_os = "linux")]
        {
            Ok("/var/run/tailscale/tailscaled.sock".to_string())
        }

        #[cfg(target_os = "macos")]
        {
            Self::get_macos_localapi_endpoint()
        }

        #[cfg(target_os = "windows")]
        {
            // Windows uses named pipes
            Ok("\\\\.\\pipe\\ProtectedPrefix\\Administrators\\Tailscale\\tailscaled".to_string())
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(PlatformError::UnsupportedOS(
                std::env::consts::OS.to_string(),
            ))
        }
    }

    /// Get macOS LocalAPI endpoint with credentials
    #[cfg(target_os = "macos")]
    fn get_macos_localapi_endpoint() -> Result<String, PlatformError> {
        // Try MacSys (standalone) method first
        if let Ok(endpoint) = Self::read_macsys_same_user_proof() {
            return Ok(endpoint);
        }

        // Try macOS App Store method
        if let Ok(endpoint) = Self::read_macos_same_user_proof() {
            return Ok(endpoint);
        }

        Err(PlatformError::SocketNotFound(
            "No Tailscale LocalAPI credentials found".to_string(),
        ))
    }

    /// Read MacSys standalone credentials from /Library/Tailscale/
    #[cfg(target_os = "macos")]
    fn read_macsys_same_user_proof() -> Result<String, PlatformError> {
        use std::fs;

        let shared_dir = "/Library/Tailscale";

        // Read port from symlink
        let port_str = fs::read_link(format!("{}/ipnport", shared_dir))
            .map_err(|_| PlatformError::SocketNotFound("ipnport symlink not found".to_string()))?
            .to_string_lossy()
            .to_string();

        // Read token from sameuserproof file
        let auth_content = fs::read_to_string(format!("{}/sameuserproof-{}", shared_dir, port_str))
            .map_err(|_| {
                PlatformError::SocketNotFound("sameuserproof file not found".to_string())
            })?;

        let token = auth_content.trim();
        if token.is_empty() {
            return Err(PlatformError::SocketNotFound(
                "empty auth token".to_string(),
            ));
        }

        // Test connection
        let addr = format!("127.0.0.1:{}", port_str);
        if let Err(_) = std::net::TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            std::time::Duration::from_secs(1),
        ) {
            return Err(PlatformError::SocketNotFound(
                "port not reachable".to_string(),
            ));
        }

        Ok(format!("tcp://127.0.0.1:{}:{}", port_str, token))
    }

    /// Read macOS App Store credentials using lsof
    #[cfg(target_os = "macos")]
    fn read_macos_same_user_proof() -> Result<String, PlatformError> {
        use std::process::Command;

        let output = Command::new("lsof")
            .args(&[
                "-n",                                        // numeric sockets
                "-a",                                        // logical AND
                &format!("-u{}", unsafe { libc::getuid() }), // current user only
                "-c",
                "IPNExtension", // IPNExtension process
                "-F",           // machine-readable
            ])
            .output()
            .map_err(|_| PlatformError::SocketNotFound("lsof command failed".to_string()))?;

        if !output.status.success() {
            return Err(PlatformError::SocketNotFound("lsof failed".to_string()));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let search_pattern = ".tailscale.ipn.macos/sameuserproof-";

        for line in output_str.lines() {
            if let Some(pos) = line.find(search_pattern) {
                let suffix = &line[pos + search_pattern.len()..];
                let parts: Vec<&str> = suffix.splitn(2, '-').collect();
                if parts.len() == 2 {
                    let (port_str, token) = (parts[0], parts[1]);
                    if let Ok(_port) = port_str.parse::<u16>() {
                        return Ok(format!("tcp://127.0.0.1:{}:{}", port_str, token));
                    }
                }
            }
        }

        Err(PlatformError::SocketNotFound(
            "No IPNExtension sameuserproof found".to_string(),
        ))
    }
}
