use crate::config::{Protocol, ProviderConfig, ServiceInfo};
use crate::tailscale::{PeerStatus, TailscaleClient};
use crate::traefik::{
    DynamicConfig, HttpConfig, LoadBalancer, Router, Server, Service, TcpConfig, TcpLoadBalancer,
    TcpRouter, TcpServer, TcpService, UdpConfig, UdpLoadBalancer, UdpRouter, UdpServer, UdpService,
};
use std::collections::HashMap;
use tracing::{info, warn};

pub struct TraefikProvider {
    pub tailscale_client: TailscaleClient,
    config: ProviderConfig,
}

impl TraefikProvider {
    pub fn new(config: ProviderConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tailscale_client = if let Some(socket_path) = &config.tailscale_socket_path {
            TailscaleClient::with_socket_path(socket_path.clone())?
        } else {
            TailscaleClient::new()?
        };

        Ok(Self {
            tailscale_client,
            config,
        })
    }

    /// Generate Traefik dynamic configuration from Tailscale status
    pub async fn generate_config(
        &self,
    ) -> Result<DynamicConfig, Box<dyn std::error::Error + Send + Sync>> {
        info!("Fetching Tailscale status");
        let status = self.tailscale_client.get_status().await?;

        let peer_count = status.peers.as_ref().map(|p| p.len()).unwrap_or(0);
        info!("Generating Traefik configuration for {} peers", peer_count);

        let mut http_services = HashMap::new();
        let mut http_routers = HashMap::new();
        let mut tcp_services = HashMap::new();
        let mut tcp_routers = HashMap::new();
        let mut udp_services = HashMap::new();
        let mut udp_routers = HashMap::new();

        // Process each online peer
        let Some(peers) = &status.peers else {
            warn!("No peers available in status");
            return Ok(DynamicConfig {
                http: Some(HttpConfig {
                    routers: HashMap::new(),
                    services: HashMap::new(),
                    middlewares: HashMap::new(),
                }),
                tcp: Some(TcpConfig {
                    routers: HashMap::new(),
                    services: HashMap::new(),
                }),
                udp: Some(UdpConfig {
                    routers: HashMap::new(),
                    services: HashMap::new(),
                }),
            });
        };

        for (_peer_key, peer_opt) in peers {
            let Some(peer) = peer_opt else { continue };
            if !self.should_include_peer(peer) {
                continue;
            }

            // Get all services from this peer's tags
            let service_infos = self.extract_service_infos_from_peer(peer);

            for service_info in service_infos {
                let service_name = self.generate_service_name_from_info(peer, &service_info);
                let router_name = self.generate_router_name_from_info(peer, &service_info);

                match service_info.protocol {
                    Protocol::Http => {
                        if let Some(service) =
                            self.create_http_service_from_peer(peer, &service_info)
                        {
                            http_services.insert(service_name.clone(), service);
                            if let Some(router) =
                                self.create_http_router_for_peer(peer, &service_info, &service_name)
                            {
                                http_routers.insert(router_name, router);
                            }
                        }
                    }
                    Protocol::Tcp => {
                        if let Some(service) =
                            self.create_tcp_service_from_peer(peer, &service_info)
                        {
                            tcp_services.insert(service_name.clone(), service);
                            if let Some(router) =
                                self.create_tcp_router_for_peer(peer, &service_info, &service_name)
                            {
                                tcp_routers.insert(router_name, router);
                            }
                        }
                    }
                    Protocol::Udp => {
                        if let Some(service) =
                            self.create_udp_service_from_peer(peer, &service_info)
                        {
                            udp_services.insert(service_name.clone(), service);
                            if let Some(router) =
                                self.create_udp_router_for_peer(peer, &service_info, &service_name)
                            {
                                udp_routers.insert(router_name, router);
                            }
                        }
                    }
                }
            }
        }

        let http_config = if http_services.is_empty() && http_routers.is_empty() {
            None
        } else {
            Some(HttpConfig {
                services: http_services,
                routers: http_routers,
                middlewares: HashMap::new(),
            })
        };

        let tcp_config = if tcp_services.is_empty() && tcp_routers.is_empty() {
            None
        } else {
            Some(TcpConfig {
                services: tcp_services,
                routers: tcp_routers,
            })
        };

        let udp_config = if udp_services.is_empty() && udp_routers.is_empty() {
            None
        } else {
            Some(UdpConfig {
                services: udp_services,
                routers: udp_routers,
            })
        };

        Ok(DynamicConfig {
            http: http_config,
            tcp: tcp_config,
            udp: udp_config,
        })
    }

    /// Extract all service infos from a peer's tags
    fn extract_service_infos_from_peer(&self, peer: &PeerStatus) -> Vec<ServiceInfo> {
        let mut service_infos = Vec::new();

        if let Some(peer_tags) = &peer.tags {
            if let Some(include_tags) = &self.config.include_tags {
                for peer_tag in peer_tags {
                    if let Some(service_info) = self.config.parse_service_info_from_tag(peer_tag) {
                        // Check if this service is in the include list
                        if include_tags.contains(&service_info.name) {
                            service_infos.push(service_info);
                        }
                    }
                }
            } else {
                // No include filter - include all parseable tags
                for peer_tag in peer_tags {
                    if let Some(service_info) = self.config.parse_service_info_from_tag(peer_tag) {
                        service_infos.push(service_info);
                    }
                }
            }
        } else if self.config.include_tags.is_none() {
            // No tags on peer, but no filter either - use default service
            service_infos.push(ServiceInfo {
                name: "default".to_string(),
                port: Some(self.config.default_port),
                protocol: self.config.default_protocol.clone(),
                scheme: self.config.default_scheme.clone(),
            });
        }

        // Check tag-service mapping for additional services
        if let Some(mapping) = &self.config.tag_service_mapping {
            if let Some(peer_tags) = &peer.tags {
                for peer_tag in peer_tags {
                    // Remove "tag:" prefix if present
                    let clean_tag = peer_tag.strip_prefix("tag:").unwrap_or(peer_tag);
                    if let Some(mapped_service) = mapping.get(clean_tag) {
                        // Check if this service should be included
                        if let Some(include_tags) = &self.config.include_tags {
                            if include_tags.contains(&mapped_service.name) {
                                service_infos.push(mapped_service.clone());
                            }
                        } else {
                            service_infos.push(mapped_service.clone());
                        }
                    }
                }
            }
        }

        service_infos
    }

    /// Generate service name from service info
    fn generate_service_name_from_info(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
    ) -> String {
        let hostname_safe = peer.hostname.to_lowercase().replace(['.', '_'], "-");
        if service_info.name == "default" {
            format!("tailscale-{}", hostname_safe)
        } else {
            format!("tailscale-{}-{}", hostname_safe, service_info.name)
        }
    }

    /// Generate router name from service info
    fn generate_router_name_from_info(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
    ) -> String {
        let service_name = self.generate_service_name_from_info(peer, service_info);
        format!("{}-router", service_name)
    }

    /// Check if peer should be included in Traefik configuration
    fn should_include_peer(&self, peer: &PeerStatus) -> bool {
        // Only include online peers
        if !peer.online.unwrap_or(false) {
            return false;
        }

        // Skip exit nodes if configured
        if self.config.exclude_exit_nodes && peer.exit_node {
            return false;
        }

        // Check if peer matches include/exclude filters
        if let Some(include_tags) = &self.config.include_tags {
            // Check if peer has any of the required tags
            if let Some(peer_tags) = &peer.tags {
                let has_matching_tag = include_tags.iter().any(|tag| {
                    peer_tags.iter().any(|peer_tag| {
                        // Remove "tag:" prefix before comparison
                        let clean_peer_tag = peer_tag.strip_prefix("tag:").unwrap_or(peer_tag);
                        clean_peer_tag.contains(tag)
                    })
                });
                if !has_matching_tag {
                    return false;
                }
            } else {
                // Peer has no tags but we require tags - exclude it
                return false;
            }
        }

        if let Some(exclude_hostnames) = &self.config.exclude_hostnames {
            if exclude_hostnames.contains(&peer.hostname) {
                return false;
            }
        }

        // Check if peer is too inactive based on max_inactive_seconds
        if let Some(max_inactive) = self.config.max_inactive_seconds {
            use chrono::{TimeZone, Utc};
            let now = Utc::now();
            let epoch = Utc.timestamp_opt(0, 0).unwrap();

            // If last_write is epoch time (zero), treat as "never written"
            if peer.last_write == epoch {
                return false; // Exclude peers that have never written
            }

            let inactive_duration = now.signed_duration_since(peer.last_write);
            if inactive_duration.num_seconds() > max_inactive {
                return false;
            }
        }

        // Check if peer matches include_os filter
        if let Some(include_os) = &self.config.include_os {
            if !include_os.contains(&peer.os) {
                return false;
            }
        }

        // Exclude expired peers if configured
        if self.config.exclude_expired {
            if peer.expired.unwrap_or(false) {
                return false;
            }
        }

        true
    }


    /// Create HTTP service from Tailscale peer
    fn create_http_service_from_peer(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
    ) -> Option<Service> {
        if peer.tailscale_ips.is_empty() {
            warn!("Peer {} has no Tailscale IPs", peer.hostname);
            return None;
        }

        // Use the first Tailscale IP
        let ip = &peer.tailscale_ips[0];
        let port = service_info.port.unwrap_or(self.config.default_port);

        let server = Server {
            url: format!("{}://{}:{}", service_info.scheme, ip, port),
            weight: Some(1),
        };

        Some(Service {
            load_balancer: LoadBalancer {
                servers: vec![server],
                health_check: self.config.health_check_path.as_ref().map(|path| {
                    crate::traefik::HealthCheck {
                        path: path.clone(),
                        interval: Some("30s".to_string()),
                        timeout: Some("5s".to_string()),
                    }
                }),
            },
        })
    }

    /// Create HTTP router for a peer
    fn create_http_router_for_peer(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
        service_name: &str,
    ) -> Option<Router> {
        // Check if this service has a custom domain mapping
        let rule = if let Some(domain_mapping) = &self.config.service_domain_mapping {
            if let Some(domain) = domain_mapping.get(&service_info.name) {
                // Use custom domain for this service
                format!("Host(`{}`)", domain)
            } else {
                // No custom domain, use default behavior
                self.generate_default_host_rule(peer)
            }
        } else {
            // No domain mapping configured, use default behavior
            self.generate_default_host_rule(peer)
        };

        Some(Router {
            rule,
            service: service_name.to_string(),
            middlewares: None,
            priority: None,
            tls: None,
        })
    }

    /// Generate default host rule - wildcard to accept all requests
    fn generate_default_host_rule(&self, _peer: &PeerStatus) -> String {
        "HostRegexp(`.*`)".to_string()
    }

    /// Create TCP service from Tailscale peer
    fn create_tcp_service_from_peer(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
    ) -> Option<TcpService> {
        if peer.tailscale_ips.is_empty() {
            warn!("Peer {} has no Tailscale IPs", peer.hostname);
            return None;
        }

        let ip = &peer.tailscale_ips[0];
        let port = service_info.port.unwrap_or(self.config.default_port);

        let server = TcpServer {
            address: format!("{}:{}", ip, port),
            weight: Some(1),
        };

        Some(TcpService {
            load_balancer: TcpLoadBalancer {
                servers: vec![server],
            },
        })
    }

    /// Create TCP router for a peer
    fn create_tcp_router_for_peer(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
        service_name: &str,
    ) -> Option<TcpRouter> {
        // Check if this service has a custom domain mapping for SNI
        let rule = if let Some(domain_mapping) = &self.config.service_domain_mapping {
            if let Some(domain) = domain_mapping.get(&service_info.name) {
                // Use HostSNI with custom domain (for TLS-enabled TCP services)
                format!("HostSNI(`{}`)", domain)
            } else {
                // No custom domain, accept all connections
                "HostSNI(`*`)".to_string()
            }
        } else {
            // No domain mapping, accept all connections
            "HostSNI(`*`)".to_string()
        };

        Some(TcpRouter {
            rule,
            service: service_name.to_string(),
            tls: None,
        })
    }

    /// Create UDP service from Tailscale peer
    fn create_udp_service_from_peer(
        &self,
        peer: &PeerStatus,
        service_info: &ServiceInfo,
    ) -> Option<UdpService> {
        if peer.tailscale_ips.is_empty() {
            warn!("Peer {} has no Tailscale IPs", peer.hostname);
            return None;
        }

        let ip = &peer.tailscale_ips[0];
        let port = service_info.port.unwrap_or(self.config.default_port);

        let server = UdpServer {
            address: format!("{}:{}", ip, port),
            weight: Some(1),
        };

        Some(UdpService {
            load_balancer: UdpLoadBalancer {
                servers: vec![server],
            },
        })
    }

    /// Create UDP router for a peer
    fn create_udp_router_for_peer(
        &self,
        _peer: &PeerStatus,
        _service_info: &ServiceInfo,
        service_name: &str,
    ) -> Option<UdpRouter> {
        // UDP routers are simple - just point to service
        Some(UdpRouter {
            service: service_name.to_string(),
        })
    }

    /// Test connectivity to Tailscale daemon
    pub async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Testing connection to Tailscale daemon");
        self.tailscale_client.test_connection().await?;
        info!("Successfully connected to Tailscale daemon");
        Ok(())
    }
}
