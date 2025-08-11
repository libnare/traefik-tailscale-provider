use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DynamicConfig {
    pub http: Option<HttpConfig>,
    pub tcp: Option<TcpConfig>,
    pub udp: Option<UdpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HttpConfig {
    pub routers: HashMap<String, Router>,
    pub services: HashMap<String, Service>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub middlewares: HashMap<String, Middleware>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpConfig {
    pub routers: HashMap<String, TcpRouter>,
    pub services: HashMap<String, TcpService>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UdpConfig {
    pub routers: HashMap<String, UdpRouter>,
    pub services: HashMap<String, UdpService>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Router {
    pub rule: String,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middlewares: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Service {
    #[serde(rename = "loadBalancer")]
    pub load_balancer: LoadBalancer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoadBalancer {
    pub servers: Vec<Server>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthCheck {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Middleware {
    // Common middlewares - can be extended as needed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HeadersMiddleware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryMiddleware>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeadersMiddleware {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_request_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_response_headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryMiddleware {
    pub attempts: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TlsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_resolver: Option<String>,
}

// TCP Router and Service types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpRouter {
    pub rule: String,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TcpTlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpService {
    #[serde(rename = "loadBalancer")]
    pub load_balancer: TcpLoadBalancer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpLoadBalancer {
    pub servers: Vec<TcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpServer {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TcpTlsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passthrough: Option<bool>,
}

// UDP Router and Service types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UdpRouter {
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UdpService {
    #[serde(rename = "loadBalancer")]
    pub load_balancer: UdpLoadBalancer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UdpLoadBalancer {
    pub servers: Vec<UdpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UdpServer {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}
