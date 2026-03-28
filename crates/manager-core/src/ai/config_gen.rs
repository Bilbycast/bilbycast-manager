// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

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
1. RTP - Receive RTP packets over UDP (unicast or multicast). Requires valid RTP v2 headers.
   - bind_addr: local address to bind (e.g., "0.0.0.0:5000" or multicast "239.1.1.1:5000")
   - Optional: FEC decode (SMPTE 2022-1), source filtering, payload type filtering, rate limiting

2. UDP - Receive raw UDP datagrams (no RTP header required). For raw MPEG-TS over UDP from OBS, ffmpeg, srt-live-transmit, etc.
   - bind_addr: local address to bind (e.g., "0.0.0.0:5000" or multicast "239.1.1.1:5000")
   - Optional: interface_addr for multicast interface

3. SRT - Receive RTP over SRT (Secure Reliable Transport)
   - Modes: caller (initiates connection), listener (waits for connection), rendezvous (both sides connect)
   - local_addr: local bind address
   - remote_addr: required for caller/rendezvous modes
   - latency_ms: SRT latency in milliseconds (default 120, sets both receiver and sender latency)
   - recv_latency_ms: Optional receiver-side latency override in ms (overrides latency_ms for receiver)
   - peer_latency_ms: Optional sender/peer-side latency override in ms (overrides latency_ms for sender)
   - peer_idle_timeout_secs: seconds before connection is considered dead if no data (default 30, use higher for broadcast)
   - Optional: AES encryption (passphrase + key length 16/24/32 + crypto_mode "aes-ctr" or "aes-gcm"). Default cipher is AES-CTR. AES-GCM provides authenticated encryption (integrity + confidentiality) but requires libsrt >= 1.5.2 on the peer and only supports AES-128/256 (not AES-192).
   - Optional: max_rexmit_bw — maximum retransmission bandwidth in bytes/sec (-1 = unlimited, 0 = disable retransmissions, >0 = cap). Uses Token Bucket shaper to prevent retransmissions from starving live data on lossy links.
   - Optional: packet_filter — SRT FEC (Forward Error Correction) config string. Format: "fec,cols:10,rows:5,layout:staircase,arq:onreq". Cols = row group size (number of columns), rows = column group size (1 = row-only FEC). Layout: "even" or "staircase" (default). ARQ modes: "always" (ARQ+FEC parallel), "onreq" (FEC first, then ARQ, default), "never" (FEC only). Both sides must agree on parameters during handshake.
   - Advanced optional: max_bw (max bandwidth bytes/s, 0=unlimited), input_bw (estimated input rate bytes/s, 0=auto), overhead_bw (overhead %, 5-100, default 25), enforced_encryption (bool, reject unencrypted peers), connect_timeout_secs (default 3), flight_flag_size (flow window pkts, default 25600), send_buffer_size/recv_buffer_size (buffer pkts, default 8192), payload_size (bytes per SRT packet, default 1316), ip_tos (DSCP 0-255), retransmit_algo ("default" or "reduced"), send_drop_delay (ms, -1=off), loss_max_ttl (reorder tolerance, 0=adaptive), km_refresh_rate/km_pre_announce (key rotation packets)
   - Optional: SMPTE 2022-7 redundancy — add "redundancy" object with leg 2 config (mode, local_addr, remote_addr, latency_ms, passphrase, aes_key_len, crypto_mode, max_rexmit_bw, and all advanced options above). Merges two independent SRT legs for hitless protection switching.

4. RTMP - Accept publish connections from OBS, ffmpeg, etc.
   - listen_addr: address to listen on (e.g., "0.0.0.0:1935")
   - app: RTMP application name (default "live")
   - Optional: stream_key for authentication

OUTPUT TYPES:
1. RTP - Send RTP-wrapped packets over UDP (with RTP headers)
   - dest_addr: destination (e.g., "192.168.1.100:5004")
   - Optional: FEC encode (SMPTE 2022-1), DSCP QoS marking

2. UDP - Send raw MPEG-TS over UDP (no RTP headers, 7×188-byte datagrams)
   - dest_addr: destination (e.g., "192.168.1.100:5004")
   - Optional: bind_addr, interface_addr (for multicast), DSCP QoS marking
   - Strips RTP headers if input is RTP-wrapped. Standard IP/TS transport for ffplay, VLC, multicast.

3. SRT - Send RTP over SRT
   - Same connection options as SRT input (mode, local_addr, remote_addr, latency_ms, passphrase, aes_key_len, crypto_mode, max_rexmit_bw, packet_filter, stream_id, and all advanced options: max_bw, input_bw, overhead_bw, enforced_encryption, connect_timeout_secs, flight_flag_size, send_buffer_size, recv_buffer_size, payload_size, ip_tos, retransmit_algo, send_drop_delay, loss_max_ttl, km_refresh_rate, km_pre_announce)
   - Optional: packet_filter — SRT FEC config string (same format as SRT input). Both sides must agree on FEC parameters.
   - Optional: SMPTE 2022-7 redundancy — add "redundancy" object with leg 2 config for dual-leg sending. Duplicates every packet to two independent SRT paths. Example:
     "redundancy": { "mode": "caller", "local_addr": "0.0.0.0:0", "remote_addr": "backup-host:9001", "latency_ms": 120 }

3. RTMP/RTMPS - Publish to streaming platforms (Twitch, YouTube Live)
   - dest_url: RTMP/RTMPS URL
   - stream_key: authentication key
   - Auto-reconnect with configurable delay

4. HLS - HTTP Live Streaming ingest
   - ingest_url: HLS ingest endpoint
   - segment_duration_secs: target segment length (default 2.0)

5. WebRTC/WHIP Output - push media via WHIP client or serve via WHEP server
   - mode: "whip_client" (push to endpoint) or "whep_server" (serve viewers)
   - WHIP client: whip_url required, optional bearer_token
   - WHEP server: optional max_viewers (default 10), bearer_token
   - Optional: video_only mode, public_ip for NAT

INPUT TYPES also support WebRTC:
- WebRTC/WHIP Input: accept contributions from OBS/browsers via WHIP (type: "webrtc")
  - Optional: bearer_token, public_ip, stun_server, video_only
  - Publishers POST SDP offers to /api/v1/flows/{flow_id}/whip
- WHEP Input: pull media from external WHEP server (type: "whep")
  - whep_url required, optional bearer_token, video_only

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
    "type": "rtp | udp | srt | rtmp | rtsp | webrtc | whep",
    // For RTP: "bind_addr", optionally "interface_addr", "fec_decode", "allowed_sources", etc.
    // For UDP: "bind_addr", optionally "interface_addr" (no FEC or RTP-specific features)
    // For SRT: "mode", "local_addr", optionally "remote_addr", "latency_ms", "recv_latency_ms", "peer_latency_ms", "peer_idle_timeout_secs", "passphrase", "aes_key_len", "crypto_mode", "max_rexmit_bw", "packet_filter" (FEC), "redundancy": {...}
    // For RTMP: "listen_addr", optionally "app", "stream_key"
    // For RTSP: "rtsp_url", optionally "username", "password", "transport" (tcp|udp), "reconnect_delay_secs"
    // For WebRTC (WHIP server): optionally "bearer_token", "video_only", "public_ip", "stun_server" — publishers POST SDP to /api/v1/flows/{id}/whip
    // For WHEP (client): "whep_url", optionally "bearer_token", "video_only" — pulls media from external WHEP server
  },
  "outputs": [
    {
      "type": "rtp | udp | srt | rtmp | hls | webrtc",
      "id": "string - unique output ID within flow",
      "name": "string - human-readable name",
      // For SRT: "mode", "local_addr", optionally "remote_addr", "latency_ms", "recv_latency_ms", "peer_latency_ms", "peer_idle_timeout_secs", "passphrase", "aes_key_len", "crypto_mode", "max_rexmit_bw", "packet_filter" (FEC), "stream_id", "redundancy": {...}
      // Other types: fields as documented above
    }
  ]
}
"#;
