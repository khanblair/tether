//! Public IP address classification for SSRF protection.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns whether an address is suitable for a public web request.
#[must_use]
pub fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => public_v4(ip),
        IpAddr::V6(ip) => public_v6(ip),
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    let first = octets[0];
    let second = octets[1];
    if first == 0 || first == 10 || first == 127 || first >= 224 {
        return false;
    }
    if first == 100 && (64..=127).contains(&second) {
        return false;
    }
    if first == 169 && second == 254 {
        return false;
    }
    if first == 172 && (16..=31).contains(&second) {
        return false;
    }
    if first == 192 && second == 168 {
        return false;
    }
    if first == 192 && second == 0 && octets[2] == 0 {
        return false;
    }
    if first == 192 && second == 0 && octets[2] == 2 {
        return false;
    }
    if first == 198 && second == 51 && octets[2] == 100 {
        return false;
    }
    if first == 198 && (18..=19).contains(&second) {
        return false;
    }
    if first == 203 && second == 0 && octets[2] == 113 {
        return false;
    }
    true
}

fn public_v6(ip: Ipv6Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
        return false;
    }
    let segments = ip.segments();
    if segments[0] & 0xfe00 == 0xfc00 || segments[0] & 0xffc0 == 0xfe80 {
        return false;
    }
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return false;
    }
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return public_v4(mapped);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn blocks_private_and_special_ipv4_ranges() {
        for value in [
            "0.0.0.0",
            "10.0.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.168.1.1",
            "100.64.0.1",
            "224.0.0.1",
            "192.0.2.1",
            "198.51.100.1",
            "203.0.113.1",
        ] {
            assert!(!is_public_ip(IpAddr::from_str(value).unwrap()), "{value}");
        }
    }

    #[test]
    fn blocks_ipv6_special_ranges_and_accepts_public_addresses() {
        for value in [
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!is_public_ip(IpAddr::from_str(value).unwrap()), "{value}");
        }
        for value in ["8.8.8.8", "1.1.1.1", "2001:4860:4860::8888"] {
            assert!(is_public_ip(IpAddr::from_str(value).unwrap()), "{value}");
        }
    }
}
