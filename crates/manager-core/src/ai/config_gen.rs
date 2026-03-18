use super::AiContext;

/// Build the AI context with protocol documentation and schema.
pub fn build_ai_context(node_info: Vec<super::NodeInfo>) -> AiContext {
    AiContext {
        protocol_docs: PROTOCOL_DOCS.to_string(),
        flow_config_schema: FLOW_CONFIG_SCHEMA.to_string(),
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
   - Optional: AES encryption (passphrase + key length 16/24/32), SMPTE 2022-7 redundancy

OUTPUT TYPES:
1. RTP/UDP - Send RTP packets over UDP
   - dest_addr: destination (e.g., "192.168.1.100:5004")
   - Optional: FEC encode, DSCP QoS marking

2. SRT - Send RTP over SRT (same options as SRT input)

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
"#;

pub const FLOW_CONFIG_SCHEMA: &str = r#"
{
  "id": "string - unique flow identifier",
  "name": "string - human-readable name",
  "enabled": "boolean - auto-start on startup (default true)",
  "input": {
    "type": "rtp | srt",
    // For RTP: "bind_addr", optionally "interface_addr", "fec_decode", "allowed_sources", etc.
    // For SRT: "mode", "local_addr", optionally "remote_addr", "latency_ms", "passphrase", etc.
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
