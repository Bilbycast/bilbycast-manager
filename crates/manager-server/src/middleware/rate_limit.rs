use std::net::IpAddr;
use std::time::Instant;

use dashmap::DashMap;

/// Simple in-memory rate limiter using a sliding window.
pub struct RateLimiter {
    /// Map of IP -> (request count, window start).
    windows: DashMap<IpAddr, (u32, Instant)>,
    /// Maximum requests per window.
    max_requests: u32,
    /// Window duration in seconds.
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            windows: DashMap::new(),
            max_requests,
            window_secs,
        }
    }

    /// Check if a request from the given IP is allowed.
    /// Returns true if allowed, false if rate limited.
    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut entry = self.windows.entry(ip).or_insert((0, now));

        let (count, window_start) = entry.value_mut();
        let elapsed = now.duration_since(*window_start).as_secs();

        if elapsed >= self.window_secs {
            // Window expired, reset
            *count = 1;
            *window_start = now;
            true
        } else if *count < self.max_requests {
            *count += 1;
            true
        } else {
            false
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, 60);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip)); // 4th request should be blocked
    }
}
