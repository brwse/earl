use std::net::{IpAddr, Ipv6Addr};

use anyhow::{Result, bail};

pub fn ensure_safe_ip(ip: IpAddr, allow_private_ips: bool) -> Result<()> {
    if is_blocked_ip(ip, allow_private_ips) {
        bail!("blocked potentially unsafe IP address `{ip}`");
    }
    Ok(())
}

pub fn is_blocked_ip(ip: IpAddr, allow_private_ips: bool) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            // When allow_private_ips is enabled, RFC 1918 private ranges and
            // loopback are permitted (homelab/self-hosted service use case).
            // Cloud metadata endpoints and other hazardous ranges are always blocked.
            (!allow_private_ips && (v4.is_private() || v4.is_loopback()))
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || is_ipv4_reserved_range(v4)
                || v4.is_documentation()
                || is_ipv4_shared_space(v4)
                || is_ipv4_benchmarking(v4)
                || is_ipv4_ietf_protocol_assignments(v4)
                || v4.octets() == [169, 254, 169, 254]
                || v4.octets() == [100, 100, 100, 200]
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_blocked_ip(IpAddr::V4(mapped), allow_private_ips);
            }

            // When allow_private_ips is enabled, loopback and unique-local
            // (ULA, fc00::/7) addresses are permitted.
            (!allow_private_ips && (v6.is_loopback() || v6.is_unique_local() || is_ipv6_site_local(v6)))
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unicast_link_local()
                || is_ipv6_documentation(v6)
                || is_ipv6_metadata(v6)
        }
    }
}

fn is_ipv4_shared_space(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000
}

fn is_ipv4_reserved_range(ip: std::net::Ipv4Addr) -> bool {
    ip.octets()[0] >= 240
}

fn is_ipv4_benchmarking(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)
}

fn is_ipv4_ietf_protocol_assignments(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 192 && octets[1] == 0 && octets[2] == 0
}

fn is_ipv6_documentation(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
}

fn is_ipv6_site_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfec0
}

fn is_ipv6_metadata(ip: Ipv6Addr) -> bool {
    // Common metadata aliases used by cloud providers.
    ip == Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254)
        || ip == Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xa9fe, 0xa9fe)
}
