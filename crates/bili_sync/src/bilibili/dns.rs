use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use hickory_resolver::config::{LookupIpStrategy, NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::{Resolver, TokioResolver};
use once_cell::sync::OnceCell;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

const PUBLIC_DNS_PORT: u16 = 53;
const PUBLIC_DNS_TRUST_NEGATIVE_RESPONSES: bool = true;
const PUBLIC_DNS_TIMEOUT: Duration = Duration::from_secs(2);
const PUBLIC_DNS_ATTEMPTS: usize = 2;
const PUBLIC_DNS_SERVERS: [IpAddr; 4] = [
    IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)),
    IpAddr::V4(Ipv4Addr::new(223, 6, 6, 6)),
    IpAddr::V4(Ipv4Addr::new(119, 29, 29, 29)),
    IpAddr::V4(Ipv4Addr::new(182, 254, 116, 116)),
];

#[derive(Debug, Clone, Default)]
pub struct FallbackDnsResolver {
    public: Arc<OnceCell<TokioResolver>>,
}

#[derive(Debug)]
struct DnsLookupError {
    host: String,
    source: String,
    detail: String,
}

#[derive(Debug)]
struct CombinedDnsLookupError {
    host: String,
    system: DnsLookupError,
    public: DnsLookupError,
}

impl Resolve for FallbackDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.clone();
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let mut system = Box::pin(resolve_with_system(host.clone()));
            let mut public = Box::pin(resolver.resolve_with_public_dns(host.clone()));
            let mut system_error = None;
            let mut public_error = None;

            loop {
                tokio::select! {
                    result = &mut system, if system_error.is_none() => {
                        match result {
                            Ok(addrs) => return Ok(box_addrs(addrs)),
                            Err(err) => {
                                debug!("system DNS lookup failed for {}: {}", err.host, err.detail);
                                system_error = Some(err);
                            }
                        }
                    }
                    result = &mut public, if public_error.is_none() => {
                        match result {
                            Ok(addrs) => return Ok(box_addrs(addrs)),
                            Err(err) => {
                                debug!("public DNS fallback failed for {}: {}", err.host, err.detail);
                                public_error = Some(err);
                            }
                        }
                    }
                }

                if let (Some(system), Some(public)) = (system_error.take(), public_error.take()) {
                    return Err(box_error(CombinedDnsLookupError { host, system, public }));
                }
            }
        })
    }
}

impl FallbackDnsResolver {
    async fn resolve_with_public_dns(&self, host: String) -> Result<Vec<SocketAddr>, DnsLookupError> {
        let resolver = self
            .public
            .get_or_try_init(new_public_resolver)
            .map_err(|err| DnsLookupError {
                host: host.clone(),
                source: "public".to_owned(),
                detail: format!("初始化公共 DNS 解析器失败: {err}"),
            })?;
        let lookup = resolver.lookup_ip(host.as_str()).await.map_err(|err| DnsLookupError {
            host: host.clone(),
            source: "public".to_owned(),
            detail: err.to_string(),
        })?;
        let addrs = lookup.into_iter().map(|ip| SocketAddr::new(ip, 0)).collect::<Vec<_>>();
        if addrs.is_empty() {
            return Err(DnsLookupError {
                host,
                source: "public".to_owned(),
                detail: "公共 DNS 未返回任何地址".to_owned(),
            });
        }
        Ok(addrs)
    }
}

async fn resolve_with_system(host: String) -> Result<Vec<SocketAddr>, DnsLookupError> {
    let addrs = tokio::net::lookup_host((host.as_str(), 0))
        .await
        .map_err(|err| DnsLookupError {
            host: host.clone(),
            source: "system".to_owned(),
            detail: err.to_string(),
        })?;
    let addrs = addrs.collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err(DnsLookupError {
            host,
            source: "system".to_owned(),
            detail: "系统 DNS 未返回任何地址".to_owned(),
        });
    }
    Ok(addrs)
}

fn new_public_resolver() -> Result<TokioResolver, DnsLookupError> {
    let name_servers = NameServerConfigGroup::from_ips_clear(
        &PUBLIC_DNS_SERVERS,
        PUBLIC_DNS_PORT,
        PUBLIC_DNS_TRUST_NEGATIVE_RESPONSES,
    );
    let config = ResolverConfig::from_parts(None, vec![], name_servers);
    let mut opts = ResolverOpts::default();
    opts.ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
    opts.timeout = PUBLIC_DNS_TIMEOUT;
    opts.attempts = PUBLIC_DNS_ATTEMPTS;
    Ok(
        Resolver::builder_with_config(config, TokioConnectionProvider::default())
            .with_options(opts)
            .build(),
    )
}

fn box_addrs(addrs: Vec<SocketAddr>) -> Addrs {
    Box::new(addrs.into_iter())
}

fn box_error(err: impl Error + Send + Sync + 'static) -> Box<dyn Error + Send + Sync> {
    Box::new(err)
}

impl fmt::Display for DnsLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} DNS 解析失败（{}）：{}", self.source, self.host, self.detail)
    }
}

impl Error for DnsLookupError {}

impl fmt::Display for CombinedDnsLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "域名 {} 解析失败；系统 DNS：{}；公共 DNS 回退：{}",
            self.host, self.system.detail, self.public.detail
        )
    }
}

impl Error for CombinedDnsLookupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.system)
    }
}
