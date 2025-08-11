use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use utoipa::ToSchema;

// Newtype wrappers for type safety matching Go types
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct StableNodeID(pub String);

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NodePublic(pub String);

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct UserID(pub i64);

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NodeCapability(pub String);

// Following Tailscale's Go implementation, capabilities map to arrays of JSON values
// Use Option<Vec<serde_json::Value>> to handle null values, similar to Go's []RawMessage
pub type NodeCapMap = HashMap<NodeCapability, Option<Vec<serde_json::Value>>>;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Status {
    #[serde(rename = "Version")]
    pub version: String,

    #[serde(rename = "TUN")]
    pub tun: bool,

    #[serde(rename = "BackendState")]
    pub backend_state: String,

    #[serde(rename = "HaveNodeKey", skip_serializing_if = "Option::is_none")]
    pub have_node_key: Option<bool>,

    #[serde(rename = "AuthURL")]
    pub auth_url: String,

    #[serde(rename = "TailscaleIPs")]
    pub tailscale_ips: Vec<String>,

    #[serde(rename = "Self")]
    pub self_peer: Option<PeerStatus>,

    #[serde(rename = "ExitNodeStatus", skip_serializing_if = "Option::is_none")]
    pub exit_node_status: Option<ExitNodeStatus>,

    #[serde(rename = "Health")]
    pub health: Vec<String>,

    #[serde(rename = "MagicDNSSuffix")]
    pub magic_dns_suffix: String,

    #[serde(rename = "CurrentTailnet")]
    pub current_tailnet: Option<TailnetStatus>,

    #[serde(rename = "CertDomains")]
    pub cert_domains: Option<Vec<String>>,

    #[serde(rename = "Peer")]
    #[schema(value_type = Object)]
    pub peers: Option<HashMap<NodePublic, Option<PeerStatus>>>,

    #[serde(rename = "User")]
    pub user: Option<HashMap<UserID, UserProfile>>,

    #[serde(rename = "ClientVersion")]
    pub client_version: Option<ClientVersion>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PeerStatus {
    #[serde(rename = "ID")]
    pub id: StableNodeID,

    #[serde(rename = "PublicKey")]
    pub public_key: NodePublic,

    #[serde(rename = "HostName")]
    pub hostname: String,

    #[serde(rename = "DNSName")]
    pub dns_name: String,

    #[serde(rename = "OS")]
    pub os: String,

    #[serde(rename = "UserID")]
    pub user_id: UserID,

    #[serde(rename = "AltSharerUserID", skip_serializing_if = "Option::is_none")]
    pub alt_sharer_user_id: Option<UserID>,

    #[serde(rename = "TailscaleIPs")]
    pub tailscale_ips: Vec<String>,

    #[serde(rename = "AllowedIPs")]
    pub allowed_ips: Option<Vec<String>>,

    #[serde(rename = "PrimaryRoutes", skip_serializing_if = "Option::is_none")]
    pub primary_routes: Option<Vec<String>>,

    #[serde(rename = "Tags")]
    pub tags: Option<Vec<String>>,

    #[serde(rename = "Addrs")]
    pub addrs: Option<Vec<String>>,

    #[serde(rename = "CurAddr")]
    pub cur_addr: String,

    #[serde(rename = "Relay")]
    pub relay: String,

    #[serde(rename = "PeerRelay", default)]
    pub peer_relay: String,

    #[serde(rename = "RxBytes")]
    pub rx_bytes: i64,

    #[serde(rename = "TxBytes")]
    pub tx_bytes: i64,

    #[serde(rename = "Created")]
    pub created: DateTime<Utc>,

    #[serde(rename = "LastWrite")]
    pub last_write: DateTime<Utc>,

    #[serde(rename = "LastSeen")]
    pub last_seen: DateTime<Utc>,

    #[serde(rename = "LastHandshake")]
    pub last_handshake: DateTime<Utc>,

    #[serde(rename = "Online", skip_serializing_if = "Option::is_none")]
    pub online: Option<bool>,

    #[serde(rename = "ExitNode")]
    pub exit_node: bool,

    #[serde(rename = "ExitNodeOption")]
    pub exit_node_option: bool,

    #[serde(rename = "Active")]
    pub active: bool,

    #[serde(rename = "PeerAPIURL")]
    pub peer_api_url: Option<Vec<String>>,

    #[serde(rename = "InNetworkMap")]
    pub in_network_map: bool,

    #[serde(rename = "InMagicSock")]
    pub in_magic_sock: bool,

    #[serde(rename = "InEngine")]
    pub in_engine: bool,

    #[serde(rename = "TaildropTarget")]
    pub taildrop_target: Option<TaildropTargetStatus>,

    #[serde(rename = "NoFileSharingReason")]
    pub no_file_sharing_reason: Option<String>,

    #[serde(rename = "Capabilities", skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<NodeCapability>>,

    #[serde(rename = "CapMap", skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub cap_map: Option<NodeCapMap>,

    #[serde(rename = "sshHostKeys", skip_serializing_if = "Option::is_none")]
    pub ssh_host_keys: Option<Vec<String>>,

    #[serde(rename = "ShareeNode", skip_serializing_if = "Option::is_none")]
    pub sharee_node: Option<bool>,

    #[serde(rename = "KeyExpiry")]
    pub key_expiry: Option<DateTime<Utc>>,

    #[serde(rename = "Expired")]
    pub expired: Option<bool>,

    #[serde(rename = "Location")]
    pub location: Option<Location>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct TailnetStatus {
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "MagicDNSSuffix")]
    pub magic_dns_suffix: String,

    #[serde(rename = "MagicDNSEnabled")]
    pub magic_dns_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ExitNodeStatus {
    #[serde(rename = "ID")]
    pub id: StableNodeID,

    #[serde(rename = "Online")]
    pub online: bool,

    #[serde(rename = "TailscaleIPs")]
    pub tailscale_ips: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UserProfile {
    #[serde(rename = "ID")]
    pub id: UserID,

    #[serde(rename = "LoginName")]
    pub login_name: String,

    #[serde(rename = "DisplayName")]
    pub display_name: String,

    #[serde(rename = "ProfilePicURL")]
    pub profile_pic_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ClientVersion {
    #[serde(rename = "RunningLatest", skip_serializing_if = "Option::is_none")]
    pub running_latest: Option<bool>,

    #[serde(rename = "LatestVersion", skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,

    #[serde(
        rename = "UrgentSecurityUpdate",
        skip_serializing_if = "Option::is_none"
    )]
    pub urgent_security_update: Option<bool>,

    #[serde(rename = "Notify", skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,

    #[serde(rename = "NotifyURL", skip_serializing_if = "Option::is_none")]
    pub notify_url: Option<String>,

    #[serde(rename = "NotifyText", skip_serializing_if = "Option::is_none")]
    pub notify_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Location {
    #[serde(rename = "Country")]
    pub country: Option<String>,

    #[serde(rename = "CountryCode")]
    pub country_code: Option<String>,

    #[serde(rename = "City")]
    pub city: Option<String>,

    #[serde(rename = "CityCode")]
    pub city_code: Option<String>,

    #[serde(rename = "Latitude", skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,

    #[serde(rename = "Longitude", skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,

    #[serde(rename = "Priority", skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(from = "i32", into = "i32")]
#[repr(i32)]
pub enum TaildropTargetStatus {
    Unknown = 0,
    Available = 1,
    NoNetmapAvailable = 2,
    IpnStateNotRunning = 3,
    MissingCap = 4,
    Offline = 5,
    NoPeerInfo = 6,
    UnsupportedOS = 7,
    NoPeerAPI = 8,
    OwnedByOtherUser = 9,
}

impl From<i32> for TaildropTargetStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => TaildropTargetStatus::Unknown,
            1 => TaildropTargetStatus::Available,
            2 => TaildropTargetStatus::NoNetmapAvailable,
            3 => TaildropTargetStatus::IpnStateNotRunning,
            4 => TaildropTargetStatus::MissingCap,
            5 => TaildropTargetStatus::Offline,
            6 => TaildropTargetStatus::NoPeerInfo,
            7 => TaildropTargetStatus::UnsupportedOS,
            8 => TaildropTargetStatus::NoPeerAPI,
            9 => TaildropTargetStatus::OwnedByOtherUser,
            _ => TaildropTargetStatus::Unknown,
        }
    }
}

impl From<TaildropTargetStatus> for i32 {
    fn from(status: TaildropTargetStatus) -> Self {
        status as i32
    }
}

impl fmt::Display for TaildropTargetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaildropTargetStatus::Unknown => write!(f, "Unknown"),
            TaildropTargetStatus::Available => write!(f, "Available"),
            TaildropTargetStatus::NoNetmapAvailable => write!(f, "NoNetmapAvailable"),
            TaildropTargetStatus::IpnStateNotRunning => write!(f, "IpnStateNotRunning"),
            TaildropTargetStatus::MissingCap => write!(f, "MissingCap"),
            TaildropTargetStatus::Offline => write!(f, "Offline"),
            TaildropTargetStatus::NoPeerInfo => write!(f, "NoPeerInfo"),
            TaildropTargetStatus::UnsupportedOS => write!(f, "UnsupportedOS"),
            TaildropTargetStatus::NoPeerAPI => write!(f, "NoPeerAPI"),
            TaildropTargetStatus::OwnedByOtherUser => write!(f, "OwnedByOtherUser"),
        }
    }
}
