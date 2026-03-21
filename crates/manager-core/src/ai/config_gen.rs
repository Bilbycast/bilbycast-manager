use super::AiContext;
use crate::drivers::DriverRegistry;

/// Build the AI context with protocol documentation and schema.
/// Uses the default edge protocol docs and schema.
pub fn build_ai_context(node_info: Vec<super::NodeInfo>) -> AiContext {
    AiContext {
        protocol_docs: PROTOCOL_DOCS.to_string(),
        flow_config_schema: FLOW_CONFIG_SCHEMA.to_string(),
        node_info,
    }
}

/// Build AI context aggregated from all registered device drivers.
/// Each driver contributes its own protocol docs and config schema.
pub fn build_ai_context_from_drivers(
    registry: &DriverRegistry,
    node_info: Vec<super::NodeInfo>,
) -> AiContext {
    let mut protocol_docs = String::new();
    let mut config_schema = String::new();

    for driver in registry.all() {
        if let Some(ctx) = driver.ai_context() {
            protocol_docs.push_str(&format!(
                "\n## {} ({})\n",
                driver.display_name(),
                driver.device_type()
            ));
            protocol_docs.push_str(&ctx.protocol_docs);
            protocol_docs.push('\n');

            config_schema.push_str(&format!(
                "\n## {} config schema\n",
                driver.display_name()
            ));
            config_schema.push_str(&ctx.config_schema);
            config_schema.push('\n');
        }
    }

    // Fall back to defaults if no drivers provided context
    if protocol_docs.is_empty() {
        protocol_docs = PROTOCOL_DOCS.to_string();
    }
    if config_schema.is_empty() {
        config_schema = FLOW_CONFIG_SCHEMA.to_string();
    }

    AiContext {
        protocol_docs,
        flow_config_schema: config_schema,
        node_info,
    }
}

pub const PROTOCOL_DOCS: &str = r#"
Supported protocols for bilbycast-edge flows:

INPUT TYPES:
1. RTP/UDP - Receive RTP packets over UDP (unicast or multicast)
   - bind_addr: local address to bind (e.g., "0.0.0.0:5000" or multicast "239.1.1.1:5000")
   - Optional: FEC decode (SMPTE 2022-1), source filtering, payload type filtering, rate limiting

2. SRT - Receive RTP over SRT (Secure Reliable Transport)
   - Modes: caller (initiates connection), listener (waits for connection), rendezvous (both sides connect)
   - local_addr: local bind address
   - remote_addr: required for caller/rendezvous modes
   - latency_ms: SRT latency in milliseconds (default 120)
   - peer_idle_timeout_secs: seconds before connection is considered dead if no data (default 30, use higher for broadcast)
   - Optional: AES encryption (passphrase + key length 16/24/32)
   - Optional: SMPTE 2022-7 redundancy — add "redundancy" object with leg 2 config (mode, local_addr, remote_addr, latency_ms, passphrase, aes_key_len). Merges two independent SRT legs for hitless protection switching.

3. RTMP - Accept publish connections from OBS, ffmpeg, etc.
   - listen_addr: address to listen on (e.g., "0.0.0.0:1935")
   - app: RTMP application name (default "live")
   - Optional: stream_key for authentication

OUTPUT TYPES:
1. RTP/UDP - Send RTP packets over UDP
   - dest_addr: destination (e.g., "192.168.1.100:5004")
   - Optional: FEC encode, DSCP QoS marking

2. SRT - Send RTP over SRT
   - Same connection options as SRT input (mode, local_addr, remote_addr, latency_ms, passphrase, aes_key_len)
   - Optional: SMPTE 2022-7 redundancy — add "redundancy" object with leg 2 config for dual-leg sending. Duplicates every packet to two independent SRT paths. Example:
     "redundancy": { "mode": "caller", "local_addr": "0.0.0.0:0", "remote_addr": "backup-host:9001", "latency_ms": 120 }

3. RTMP/RTMPS - Publish to streaming platforms (Twitch, YouTube Live)
   - dest_url: RTMP/RTMPS URL
   - stream_key: authentication key
   - Auto-reconnect with configurable delay

4. HLS - HTTP Live Streaming ingest
   - ingest_url: HLS ingest endpoint
   - segment_duration_secs: target segment length (default 2.0)

5. WebRTC/WHIP - WebRTC via WHIP protocol
   - whip_url: WHIP endpoint for signaling
   - Optional: bearer token, video_only mode

IP TUNNELING:
When two edge nodes are both behind NAT firewalls and need to exchange SRT or other
UDP/TCP traffic, a tunnel can be configured through bilbycast-relay.

Tunnel modes:
1. Relay - Both edges connect to a bilbycast-relay server which forwards traffic.
   Use when both nodes are behind NAT (most common for remote production).
2. Direct - One edge connects directly to the other (requires one side to have an
   open firewall port). Use when one node has a public IP.

For SRT flows through a tunnel:
- Create a UDP tunnel between the two nodes
- Configure the SRT output on the ingress node to send to localhost:<tunnel_ingress_port>
- Configure the SRT input on the egress node to listen on localhost:<tunnel_egress_port>
- The tunnel handles NAT traversal transparently

Tunnel configuration (managed via /api/v1/tunnels):
{
  "name": "string - descriptive tunnel name",
  "protocol": "tcp | udp",
  "mode": "relay | direct",
  "ingress_node_id": "string - source edge node ID",
  "ingress_listen_port": "number - local port on ingress edge for devices to connect to",
  "egress_node_id": "string - destination edge node ID",
  "egress_forward_addr": "string - address:port on egress edge's local network to forward to",
  "relay_addr": "string - bilbycast-relay address (required for relay mode)",
  "associated_flow_ids": ["optional array of flow IDs using this tunnel"]
}
"#;

pub const FLOW_CONFIG_SCHEMA: &str = r#"
{
  "id": "string - unique flow identifier",
  "name": "string - human-readable name",
  "enabled": "boolean - auto-start on startup (default true)",
  "input": {
    "type": "rtp | srt | rtmp",
    // For RTP: "bind_addr", optionally "interface_addr", "fec_decode", "allowed_sources", etc.
    // For SRT: "mode", "local_addr", optionally "remote_addr", "latency_ms", "peer_idle_timeout_secs", "passphrase", "redundancy": {...}
    // For RTMP: "listen_addr", optionally "app", "stream_key"
  },
  "outputs": [
    {
      "type": "rtp | srt | rtmp | hls | webrtc",
      "id": "string - unique output ID within flow",
      "name": "string - human-readable name",
      // Type-specific fields as documented above
    }
  ]
}
"#;
