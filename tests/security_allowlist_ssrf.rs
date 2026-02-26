use std::net::IpAddr;
use std::str::FromStr;

use earl::security::ssrf::{ensure_safe_ip, is_blocked_ip};
use earl::template::schema::AllowRule;
use earl_core::allowlist::{ensure_url_allowed, matches_rule};
use url::Url;

fn rule() -> AllowRule {
    AllowRule {
        scheme: "https".to_string(),
        host: "api.github.com".to_string(),
        port: 443,
        path_prefix: "/search/issues".to_string(),
    }
}

#[test]
fn url_matching_all_fields_satisfies_allow_rule() {
    let url = Url::parse("https://api.github.com/search/issues?q=abc").unwrap();
    assert!(matches_rule(&url, &rule()));
}

#[test]
fn url_with_exact_path_prefix_satisfies_allow_rule() {
    let url = Url::parse("https://api.github.com/search/issues").unwrap();
    assert!(matches_rule(&url, &rule()));
}

#[test]
fn url_with_different_scheme_does_not_satisfy_allow_rule() {
    let url = Url::parse("http://api.github.com/search/issues").unwrap();
    assert!(!matches_rule(&url, &rule()));
}

#[test]
fn url_with_different_host_does_not_satisfy_allow_rule() {
    let url = Url::parse("https://example.com/search/issues").unwrap();
    assert!(!matches_rule(&url, &rule()));
}

#[test]
fn url_with_different_port_does_not_satisfy_allow_rule() {
    let url = Url::parse("https://api.github.com:8443/search/issues").unwrap();
    assert!(!matches_rule(&url, &rule()));
}

#[test]
fn url_with_unmatched_path_does_not_satisfy_allow_rule() {
    let url = Url::parse("https://api.github.com/repos/owner/repo").unwrap();
    assert!(!matches_rule(&url, &rule()));
}

#[test]
fn url_extending_path_prefix_without_separator_does_not_satisfy_allow_rule() {
    let url = Url::parse("https://api.github.com/search/issues-archive").unwrap();
    assert!(!matches_rule(&url, &rule()));
}

#[test]
fn url_matching_allowlist_rule_is_permitted() {
    let url = Url::parse("https://api.github.com/search/issues?q=abc").unwrap();
    ensure_url_allowed(&url, &[rule()]).unwrap();
}

#[test]
fn url_with_non_matching_path_is_rejected() {
    let url = Url::parse("https://api.github.com/repos/owner/repo").unwrap();
    assert!(ensure_url_allowed(&url, &[rule()]).is_err());
}

#[test]
fn url_extending_path_prefix_without_separator_is_rejected() {
    let url = Url::parse("https://api.github.com/search/issues-archive").unwrap();
    assert!(ensure_url_allowed(&url, &[rule()]).is_err());
}

#[test]
fn empty_allowlist_allows_all_urls() {
    let url = Url::parse("https://example.com/anything").unwrap();
    ensure_url_allowed(&url, &[]).unwrap();
}

#[test]
fn loopback_ipv4_is_blocked() {
    let ip = IpAddr::from_str("127.0.0.1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn loopback_ipv4_is_rejected() {
    let ip = IpAddr::from_str("127.0.0.1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn private_class_a_ip_is_blocked() {
    let ip = IpAddr::from_str("10.0.0.1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn private_class_a_ip_is_rejected() {
    let ip = IpAddr::from_str("10.0.0.1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn link_local_ipv4_is_blocked() {
    let ip = IpAddr::from_str("169.254.169.254").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn link_local_ipv4_is_rejected() {
    let ip = IpAddr::from_str("169.254.169.254").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn shared_address_space_ip_is_blocked() {
    let ip = IpAddr::from_str("100.64.0.1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn shared_address_space_ip_is_rejected() {
    let ip = IpAddr::from_str("100.64.0.1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn benchmarking_ip_is_blocked() {
    let ip = IpAddr::from_str("198.18.0.1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn benchmarking_ip_is_rejected() {
    let ip = IpAddr::from_str("198.18.0.1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn reserved_ipv4_is_blocked() {
    let ip = IpAddr::from_str("240.0.0.1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn reserved_ipv4_is_rejected() {
    let ip = IpAddr::from_str("240.0.0.1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn unspecified_ipv4_is_blocked() {
    let ip = IpAddr::from_str("0.0.0.0").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn unspecified_ipv4_is_rejected() {
    let ip = IpAddr::from_str("0.0.0.0").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn loopback_ipv6_is_blocked() {
    let ip = IpAddr::from_str("::1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn loopback_ipv6_is_rejected() {
    let ip = IpAddr::from_str("::1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn link_local_ipv6_is_blocked() {
    let ip = IpAddr::from_str("fe80::1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn link_local_ipv6_is_rejected() {
    let ip = IpAddr::from_str("fe80::1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn unique_local_ipv6_is_blocked() {
    let ip = IpAddr::from_str("fd00::1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn unique_local_ipv6_is_rejected() {
    let ip = IpAddr::from_str("fd00::1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn ipv4_mapped_ipv6_is_blocked() {
    let ip = IpAddr::from_str("::ffff:10.0.0.1").unwrap();
    assert!(is_blocked_ip(ip, false));
}

#[test]
fn ipv4_mapped_ipv6_is_rejected() {
    let ip = IpAddr::from_str("::ffff:10.0.0.1").unwrap();
    assert!(ensure_safe_ip(ip, false).is_err());
}

#[test]
fn public_ip_is_not_blocked() {
    let public = IpAddr::from_str("8.8.8.8").unwrap();
    assert!(!is_blocked_ip(public, false));
}

#[test]
fn public_ip_is_permitted() {
    let public = IpAddr::from_str("8.8.8.8").unwrap();
    ensure_safe_ip(public, false).unwrap();
}

// ── allow_private_ips = true ──────────────────────────────────────────────────

#[test]
fn private_class_a_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("10.1.2.101").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn private_class_a_permitted_when_allow_private_ips() {
    let ip = IpAddr::from_str("10.1.2.101").unwrap();
    ensure_safe_ip(ip, true).unwrap();
}

#[test]
fn private_class_b_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("172.16.0.1").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn private_class_c_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("192.168.1.1").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn loopback_ipv4_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("127.0.0.1").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn loopback_ipv6_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("::1").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn unique_local_ipv6_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("fd00::1").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn ipv4_mapped_private_ipv6_allowed_when_allow_private_ips() {
    let ip = IpAddr::from_str("::ffff:10.0.0.1").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

#[test]
fn cloud_metadata_endpoint_still_blocked_when_allow_private_ips() {
    // 169.254.169.254 must remain blocked even with allow_private_ips=true
    let ip = IpAddr::from_str("169.254.169.254").unwrap();
    assert!(is_blocked_ip(ip, true));
}

#[test]
fn link_local_ipv4_still_blocked_when_allow_private_ips() {
    // Generic link-local remains blocked (includes cloud metadata range)
    let ip = IpAddr::from_str("169.254.1.1").unwrap();
    assert!(is_blocked_ip(ip, true));
}

#[test]
fn link_local_ipv6_still_blocked_when_allow_private_ips() {
    let ip = IpAddr::from_str("fe80::1").unwrap();
    assert!(is_blocked_ip(ip, true));
}

#[test]
fn shared_address_space_still_blocked_when_allow_private_ips() {
    let ip = IpAddr::from_str("100.64.0.1").unwrap();
    assert!(is_blocked_ip(ip, true));
}

#[test]
fn public_ip_still_permitted_when_allow_private_ips() {
    let ip = IpAddr::from_str("8.8.8.8").unwrap();
    assert!(!is_blocked_ip(ip, true));
}

// ── config parsing ────────────────────────────────────────────────────────────

#[test]
fn default_network_config_has_allow_private_ips_false() {
    let cfg: earl::config::Config = toml::from_str("").unwrap();
    assert!(!cfg.network.allow_private_ips);
}

#[test]
fn allow_private_ips_parsed_from_toml() {
    let cfg: earl::config::Config = toml::from_str(
        r#"
[network]
allow_private_ips = true
"#,
    )
    .unwrap();
    assert!(cfg.network.allow_private_ips);
}
