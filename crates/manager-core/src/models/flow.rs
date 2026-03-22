use serde::{Deserialize, Serialize};

// Flow configuration types that mirror bilbycast-edge's config/models.rs.
// These define the exact JSON structure that edge nodes expect.

/// A Flow is the unit of configuration: one input fanning out to N outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub media_analysis: bool,
    pub input: InputConfig,
    pub outputs: Vec<OutputConfig>,
}

fn default_true() -> bool {
    true
}

/// Input source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputConfig {
    #[serde(rename = "rtp")]
    Rtp(RtpInputConfig),
    #[serde(rename = "srt")]
    Srt(SrtInputConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtpInputConfig {
    pub bind_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fec_decode: Option<FecConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tr07_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_sources: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_payload_types: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bitrate_mbps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrtInputConfig {
    pub mode: SrtMode,
    pub local_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_addr: Option<String>,
    #[serde(default = "default_latency")]
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aes_key_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redundancy: Option<SrtRedundancyConfig>,
}

fn default_latency() -> u64 {
    120
}

/// Output destination configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputConfig {
    #[serde(rename = "rtp")]
    Rtp(RtpOutputConfig),
    #[serde(rename = "srt")]
    Srt(SrtOutputConfig),
    #[serde(rename = "rtmp")]
    Rtmp(RtmpOutputConfig),
    #[serde(rename = "hls")]
    Hls(HlsOutputConfig),
    #[serde(rename = "webrtc")]
    Webrtc(WebrtcOutputConfig),
}

impl OutputConfig {
    pub fn id(&self) -> &str {
        match self {
            Self::Rtp(c) => &c.id,
            Self::Srt(c) => &c.id,
            Self::Rtmp(c) => &c.id,
            Self::Hls(c) => &c.id,
            Self::Webrtc(c) => &c.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Rtp(c) => &c.name,
            Self::Srt(c) => &c.name,
            Self::Rtmp(c) => &c.name,
            Self::Hls(c) => &c.name,
            Self::Webrtc(c) => &c.name,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Rtp(_) => "rtp",
            Self::Srt(_) => "srt",
            Self::Rtmp(_) => "rtmp",
            Self::Hls(_) => "hls",
            Self::Webrtc(_) => "webrtc",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtpOutputConfig {
    pub id: String,
    pub name: String,
    pub dest_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fec_encode: Option<FecConfig>,
    #[serde(default = "default_dscp")]
    pub dscp: u8,
}

fn default_dscp() -> u8 {
    46
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrtOutputConfig {
    pub id: String,
    pub name: String,
    pub mode: SrtMode,
    pub local_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_addr: Option<String>,
    #[serde(default = "default_latency")]
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aes_key_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redundancy: Option<SrtRedundancyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtmpOutputConfig {
    pub id: String,
    pub name: String,
    pub dest_url: String,
    pub stream_key: String,
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_reconnect_attempts: Option<u32>,
}

fn default_reconnect_delay() -> u64 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsOutputConfig {
    pub id: String,
    pub name: String,
    pub ingest_url: String,
    #[serde(default = "default_segment_duration")]
    pub segment_duration_secs: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default = "default_max_segments")]
    pub max_segments: usize,
}

fn default_segment_duration() -> f64 {
    2.0
}

fn default_max_segments() -> usize {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebrtcOutputConfig {
    pub id: String,
    pub name: String,
    pub whip_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
    #[serde(default)]
    pub video_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SrtMode {
    #[serde(rename = "caller")]
    Caller,
    #[serde(rename = "listener")]
    Listener,
    #[serde(rename = "rendezvous")]
    Rendezvous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrtRedundancyConfig {
    pub mode: SrtMode,
    pub local_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_addr: Option<String>,
    #[serde(default = "default_latency")]
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aes_key_len: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FecConfig {
    pub columns: u8,
    pub rows: u8,
}

/// Full edge node application config (mirrors bilbycast-edge AppConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAppConfig {
    pub version: u32,
    pub server: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<serde_json::Value>,
    #[serde(default)]
    pub flows: Vec<FlowConfig>,
}

// ── Stats types that mirror bilbycast-edge's stats/models.rs ──

/// Per-flow statistics from an edge node.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowStats {
    pub flow_id: String,
    pub flow_name: String,
    pub state: String,
    pub input: InputStats,
    pub outputs: Vec<OutputStats>,
    pub uptime_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tr101290: Option<serde_json::Value>,
    #[serde(default)]
    pub health: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdv_jitter_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_analysis: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputStats {
    pub input_type: String,
    pub state: String,
    pub packets_received: u64,
    pub bytes_received: u64,
    pub bitrate_bps: u64,
    pub packets_lost: u64,
    pub packets_filtered: u64,
    pub packets_recovered_fec: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srt_stats: Option<SrtLegStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srt_leg2_stats: Option<SrtLegStats>,
    pub redundancy_switches: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputStats {
    pub output_id: String,
    pub output_name: String,
    pub output_type: String,
    pub state: String,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub bitrate_bps: u64,
    pub packets_dropped: u64,
    pub fec_packets_sent: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srt_stats: Option<SrtLegStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srt_leg2_stats: Option<SrtLegStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SrtLegStats {
    pub state: String,
    pub rtt_ms: f64,
    pub send_rate_mbps: f64,
    pub recv_rate_mbps: f64,
    pub pkt_loss_total: i64,
    pub pkt_retransmit_total: i32,
    pub uptime_ms: i64,
}
