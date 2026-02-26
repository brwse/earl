use std::net::IpAddr;

use anyhow::{Context, Result, bail};
use hickory_resolver::TokioResolver;

use super::ssrf::ensure_safe_ip;

pub async fn resolve_and_validate_host(
    host: &str,
    allow_private_ips: bool,
) -> Result<Vec<IpAddr>> {
    let resolver = TokioResolver::builder_tokio()
        .map_err(|e| anyhow::anyhow!("resolver setup failed: {e}"))?
        .build();

    let response = resolver
        .lookup_ip(host)
        .await
        .with_context(|| format!("failed DNS resolution for host `{host}`"))?;

    let mut ips = Vec::new();
    for ip in response.iter() {
        ensure_safe_ip(ip, allow_private_ips)?;
        ips.push(ip);
    }

    if ips.is_empty() {
        bail!("host `{host}` resolved to no addresses");
    }

    Ok(ips)
}
