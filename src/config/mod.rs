use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Protocol {
    Http,
    Tcp,
    Udp,
}

impl Protocol {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            "http" | "https" => Protocol::Http,
            _ => Protocol::Http,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub port: Option<u16>,
    pub protocol: Protocol,
    pub scheme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Custom Tailscale socket path (optional)
    pub tailscale_socket_path: Option<String>,

    /// Default port to use for services when not specified
    pub default_port: u16,

    /// Exclude exit nodes from configuration
    pub exclude_exit_nodes: bool,

    /// Include only peers with specific tags
    pub include_tags: Option<Vec<String>>,

    /// Exclude peers with specific hostnames
    pub exclude_hostnames: Option<Vec<String>>,

    /// Health check path for services
    pub health_check_path: Option<String>,

    /// Update interval in seconds
    pub update_interval_seconds: u64,

    /// HTTP server port for serving dynamic configuration
    pub server_port: u16,

    /// Only include peers that have been active within this many seconds
    pub max_inactive_seconds: Option<i64>,

    /// Only include peers with specific OS types
    pub include_os: Option<Vec<String>>,

    /// Exclude peers with expired node keys
    pub exclude_expired: bool,

    /// Extract port and protocol from tag format "service-port-protocol"
    pub extract_protocol_from_tag: bool,

    /// Tag to port and protocol mapping (e.g., "db:5432:tcp,cache:6379:tcp")
    pub tag_service_mapping: Option<HashMap<String, ServiceInfo>>,

    /// Default scheme (http/https)
    pub default_scheme: String,

    /// Default protocol for services
    pub default_protocol: Protocol,

    /// Service to domain mapping (e.g., "web:app.example.net,api:api.example.net")
    pub service_domain_mapping: Option<HashMap<String, String>>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            tailscale_socket_path: None,
            default_port: 80,
            exclude_exit_nodes: true,
            include_tags: None,
            exclude_hostnames: None,
            health_check_path: Some("/health".to_string()),
            update_interval_seconds: 30,
            server_port: 8080,
            max_inactive_seconds: None, // No filtering by default
            include_os: None,           // Include all OS types by default
            exclude_expired: true,      // Exclude expired peers by default
            extract_protocol_from_tag: true,
            tag_service_mapping: None,
            default_scheme: "http".to_string(),
            default_protocol: Protocol::Http,
            service_domain_mapping: None,
        }
    }
}

impl ProviderConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            tailscale_socket_path: std::env::var("TAILSCALE_SOCKET_PATH").ok(),
            default_port: std::env::var("DEFAULT_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(80),
            exclude_exit_nodes: std::env::var("EXCLUDE_EXIT_NODES")
                .map(|s| s.to_lowercase() != "false")
                .unwrap_or(true),
            include_tags: std::env::var("INCLUDE_TAGS")
                .ok()
                .map(|s| s.split(',').map(|tag| tag.trim().to_string()).collect()),
            exclude_hostnames: std::env::var("EXCLUDE_HOSTNAMES")
                .ok()
                .map(|s| s.split(',').map(|name| name.trim().to_string()).collect()),
            health_check_path: std::env::var("HEALTH_CHECK_PATH").ok(),
            update_interval_seconds: std::env::var("UPDATE_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            server_port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            max_inactive_seconds: std::env::var("MAX_INACTIVE_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok()),
            include_os: std::env::var("INCLUDE_OS")
                .ok()
                .map(|s| s.split(',').map(|os| os.trim().to_string()).collect()),
            exclude_expired: std::env::var("EXCLUDE_EXPIRED")
                .map(|s| s.to_lowercase() != "false")
                .unwrap_or(true),
            extract_protocol_from_tag: std::env::var("EXTRACT_PROTOCOL_FROM_TAG")
                .map(|s| s.to_lowercase() != "false")
                .unwrap_or(true),
            tag_service_mapping: Self::parse_service_mapping(
                &std::env::var("TAG_SERVICE_MAPPING").unwrap_or_default(),
            ),
            default_scheme: std::env::var("DEFAULT_SCHEME").unwrap_or_else(|_| "http".to_string()),
            default_protocol: Protocol::from_str(
                &std::env::var("DEFAULT_PROTOCOL").unwrap_or_else(|_| "http".to_string()),
            ),
            service_domain_mapping: Self::parse_domain_mapping(
                &std::env::var("SERVICE_DOMAIN_MAPPING").unwrap_or_default(),
            ),
        }
    }

    /// Parse domain mapping from string format "service:domain,service2:domain2"
    fn parse_domain_mapping(mapping_str: &str) -> Option<HashMap<String, String>> {
        if mapping_str.is_empty() {
            return None;
        }

        let mut mapping = HashMap::new();
        
        for entry in mapping_str.split(',') {
            let parts: Vec<&str> = entry.trim().split(':').collect();
            if parts.len() == 2 {
                let service = parts[0].trim().to_string();
                let domain = parts[1].trim().to_string();
                mapping.insert(service, domain);
            }
        }
        
        if mapping.is_empty() {
            None
        } else {
            Some(mapping)
        }
    }

    /// Parse service mapping from string format "tag:port:protocol,tag2:port2:protocol2"
    fn parse_service_mapping(mapping_str: &str) -> Option<HashMap<String, ServiceInfo>> {
        if mapping_str.is_empty() {
            return None;
        }

        let mut mapping = HashMap::new();

        for entry in mapping_str.split(',') {
            let parts: Vec<&str> = entry.trim().split(':').collect();
            if parts.len() >= 2 {
                let tag = parts[0].trim().to_string();
                if let Ok(port) = parts[1].trim().parse::<u16>() {
                    let protocol = if parts.len() >= 3 {
                        Protocol::from_str(parts[2].trim())
                    } else {
                        Protocol::Http
                    };

                    let scheme = match protocol {
                        Protocol::Http => "http",
                        Protocol::Tcp => "tcp",
                        Protocol::Udp => "udp",
                    };

                    mapping.insert(
                        tag.clone(),
                        ServiceInfo {
                            name: tag,
                            port: Some(port),
                            protocol,
                            scheme: scheme.to_string(),
                        },
                    );
                }
            }
        }

        if mapping.is_empty() {
            None
        } else {
            Some(mapping)
        }
    }

    /// Parse service info from tag in format "service-port-protocol"
    /// Returns None if parsing fails and tag doesn't match expected format
    pub fn parse_service_info_from_tag(&self, tag: &str) -> Option<ServiceInfo> {
        // Remove "tag:" prefix if present (Tailscale API returns tags with this prefix)
        let clean_tag = tag.strip_prefix("tag:").unwrap_or(tag);
        
        if !self.extract_protocol_from_tag {
            return Some(ServiceInfo {
                name: clean_tag.to_string(),
                port: Some(self.default_port),
                protocol: self.default_protocol.clone(),
                scheme: self.default_scheme.clone(),
            });
        }

        let parts: Vec<&str> = clean_tag.split('-').collect();

        match parts.len() {
            1 => {
                // "web" → ("web", default_port, default_protocol) - simple tags are allowed
                Some(ServiceInfo {
                    name: parts[0].to_string(),
                    port: Some(self.default_port),
                    protocol: self.default_protocol.clone(),
                    scheme: self.default_scheme.clone(),
                })
            }
            2 => {
                // "service-3000" → ("service", 3000, default_protocol)
                if let Ok(port) = parts[1].parse::<u16>() {
                    Some(ServiceInfo {
                        name: parts[0].to_string(),
                        port: Some(port),
                        protocol: self.default_protocol.clone(),
                        scheme: self.default_scheme.clone(),
                    })
                } else {
                    // Port parsing failed - exclude
                    None
                }
            }
            3 => {
                // "service-3000-tcp" → ("service", 3000, tcp)
                if let Ok(port) = parts[1].parse::<u16>() {
                    let protocol = Protocol::from_str(parts[2]);
                    let scheme = match &protocol {
                        Protocol::Http => {
                            if parts[2].to_lowercase() == "https" {
                                "https"
                            } else {
                                "http"
                            }
                        }
                        Protocol::Tcp => "tcp",
                        Protocol::Udp => "udp",
                    };

                    Some(ServiceInfo {
                        name: parts[0].to_string(),
                        port: Some(port),
                        protocol,
                        scheme: scheme.to_string(),
                    })
                } else {
                    // Port parsing failed - exclude
                    None
                }
            }
            _ => {
                // For 4+ parts, try to parse last two as port-protocol
                if parts.len() >= 4 {
                    let service_parts = &parts[..parts.len() - 2];
                    let service_name = service_parts.join("-");

                    if let Ok(port) = parts[parts.len() - 2].parse::<u16>() {
                        let protocol = Protocol::from_str(parts[parts.len() - 1]);
                        let scheme = match &protocol {
                            Protocol::Http => {
                                if parts[parts.len() - 1].to_lowercase() == "https" {
                                    "https"
                                } else {
                                    "http"
                                }
                            }
                            Protocol::Tcp => "tcp",
                            Protocol::Udp => "udp",
                        };

                        return Some(ServiceInfo {
                            name: service_name,
                            port: Some(port),
                            protocol,
                            scheme: scheme.to_string(),
                        });
                    }
                }

                // Parsing failed - exclude
                None
            }
        }
    }
}
