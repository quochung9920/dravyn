use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const MAX_PROBE_ADDRESSES: usize = 4;
const MAX_CONNECT_SLICE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    #[default]
    Direct,
    Proxy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProxyScheme {
    #[default]
    Http,
    Https,
    Socks5,
}

impl ProxyScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            ProxyScheme::Http => "http",
            ProxyScheme::Https => "https",
            ProxyScheme::Socks5 => "socks5",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyConfig {
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
}

impl ProxyConfig {
    pub fn validate(&self) -> Result<(), NetworkError> {
        let host = self.host.trim();
        if host.is_empty() {
            return Err(NetworkError::new("proxy host cannot be empty"));
        }
        if host.chars().any(char::is_whitespace)
            || host.contains("//")
            || host.contains('/')
            || host.contains('\\')
        {
            return Err(NetworkError::new(
                "proxy host must be a hostname or IP address without a URL path",
            ));
        }
        if self.port == 0 {
            return Err(NetworkError::new("proxy port must be between 1 and 65535"));
        }
        Ok(())
    }

    pub fn chromium_value(&self) -> Result<String, NetworkError> {
        self.validate()?;
        let host = self.host.trim();
        let printable_host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host.to_owned()
        };
        Ok(format!(
            "{}://{}:{}",
            self.scheme.as_str(),
            printable_host,
            self.port
        ))
    }

    pub fn endpoint_label(&self) -> String {
        format!("{}://{}:{}", self.scheme.as_str(), self.host.trim(), self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    pub proxy: Option<ProxyConfig>,
}

impl NetworkConfig {
    pub fn direct() -> Self {
        Self::default()
    }

    pub fn validate(&self) -> Result<(), NetworkError> {
        match self.mode {
            NetworkMode::Direct => Ok(()),
            NetworkMode::Proxy => self
                .proxy
                .as_ref()
                .ok_or_else(|| NetworkError::new("proxy mode requires proxy settings"))?
                .validate(),
        }
    }

    pub fn chromium_argument(&self) -> Result<Option<String>, NetworkError> {
        match self.mode {
            NetworkMode::Direct => Ok(None),
            NetworkMode::Proxy => {
                let proxy = self
                    .proxy
                    .as_ref()
                    .ok_or_else(|| NetworkError::new("proxy mode requires proxy settings"))?;
                Ok(Some(format!("--proxy-server={}", proxy.chromium_value()?)))
            }
        }
    }

    pub fn endpoint_label(&self) -> Option<String> {
        self.proxy.as_ref().map(ProxyConfig::endpoint_label)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkProbeResult {
    pub mode: String,
    pub endpoint: Option<String>,
    pub valid: bool,
    pub reachable: Option<bool>,
    pub latency_ms: Option<u64>,
    pub message: String,
}

pub fn probe_network(config: &NetworkConfig, timeout: Duration) -> NetworkProbeResult {
    if let Err(error) = config.validate() {
        return NetworkProbeResult {
            mode: "invalid".to_owned(),
            endpoint: None,
            valid: false,
            reachable: None,
            latency_ms: None,
            message: error.to_string(),
        };
    }

    match config.mode {
        NetworkMode::Direct => NetworkProbeResult {
            mode: "direct".to_owned(),
            endpoint: None,
            valid: true,
            reachable: None,
            latency_ms: None,
            message: "Direct connection is configured. No proxy endpoint is required.".to_owned(),
        },
        NetworkMode::Proxy => {
            let Some(proxy) = config.proxy.as_ref() else {
                return NetworkProbeResult {
                    mode: "invalid".to_owned(),
                    endpoint: None,
                    valid: false,
                    reachable: None,
                    latency_ms: None,
                    message: "proxy settings are missing".to_owned(),
                };
            };
            let endpoint = proxy.endpoint_label();
            let addresses = match (proxy.host.as_str(), proxy.port).to_socket_addrs() {
                Ok(addresses) => addresses.take(MAX_PROBE_ADDRESSES).collect::<Vec<_>>(),
                Err(error) => {
                    return NetworkProbeResult {
                        mode: "proxy".to_owned(),
                        endpoint: Some(endpoint),
                        valid: true,
                        reachable: Some(false),
                        latency_ms: None,
                        message: format!("Failed to resolve proxy host: {error}"),
                    };
                }
            };
            if addresses.is_empty() {
                return NetworkProbeResult {
                    mode: "proxy".to_owned(),
                    endpoint: Some(endpoint),
                    valid: true,
                    reachable: Some(false),
                    latency_ms: None,
                    message: "Proxy host resolved to no usable address.".to_owned(),
                };
            }

            let started = Instant::now();
            let reachable = connect_any_with_budget(&addresses, timeout, started);
            let latency_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            NetworkProbeResult {
                mode: "proxy".to_owned(),
                endpoint: Some(endpoint),
                valid: true,
                reachable: Some(reachable),
                latency_ms: Some(latency_ms),
                message: if reachable {
                    "Proxy endpoint accepted a TCP connection within the bounded preflight budget. This proves endpoint reachability only; browser-side IP/DNS/IPv6/WebRTC exposure still requires verification.".to_owned()
                } else {
                    format!(
                        "Proxy endpoint could not be reached within the {} ms preflight budget.",
                        timeout.as_millis()
                    )
                },
            }
        }
    }
}

fn connect_any_with_budget(
    addresses: &[SocketAddr],
    timeout: Duration,
    started: Instant,
) -> bool {
    if timeout.is_zero() {
        return false;
    }

    for (index, address) in addresses.iter().enumerate() {
        let elapsed = started.elapsed();
        let Some(remaining) = timeout.checked_sub(elapsed) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }

        let attempts_left = addresses.len().saturating_sub(index).max(1) as u128;
        let fair_share_ms = (remaining.as_millis() / attempts_left).max(1);
        let slice_ms = fair_share_ms.min(MAX_CONNECT_SLICE.as_millis()).min(u64::MAX as u128);
        let connect_timeout = Duration::from_millis(slice_ms as u64);
        if TcpStream::connect_timeout(address, connect_timeout).is_ok() {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkError {
    message: String,
}

impl NetworkError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NetworkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_mode_has_no_chromium_proxy_argument() {
        assert_eq!(NetworkConfig::direct().chromium_argument().unwrap(), None);
    }

    #[test]
    fn direct_probe_is_valid_without_endpoint() {
        let probe = probe_network(&NetworkConfig::direct(), Duration::from_millis(5));
        assert!(probe.valid);
        assert_eq!(probe.mode, "direct");
        assert_eq!(probe.reachable, None);
    }

    #[test]
    fn http_proxy_builds_expected_chromium_argument() {
        let config = NetworkConfig {
            mode: NetworkMode::Proxy,
            proxy: Some(ProxyConfig {
                scheme: ProxyScheme::Http,
                host: "127.0.0.1".to_owned(),
                port: 8080,
            }),
        };
        assert_eq!(
            config.chromium_argument().unwrap().as_deref(),
            Some("--proxy-server=http://127.0.0.1:8080")
        );
        assert_eq!(
            config.endpoint_label().as_deref(),
            Some("http://127.0.0.1:8080")
        );
    }

    #[test]
    fn proxy_mode_requires_settings() {
        let config = NetworkConfig {
            mode: NetworkMode::Proxy,
            proxy: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn proxy_host_rejects_urls() {
        let proxy = ProxyConfig {
            scheme: ProxyScheme::Http,
            host: "https://proxy.example".to_owned(),
            port: 8080,
        };
        assert!(proxy.validate().is_err());
    }

    #[test]
    fn zero_budget_never_attempts_a_connection() {
        let address: SocketAddr = "127.0.0.1:9".parse().unwrap();
        assert!(!connect_any_with_budget(
            &[address],
            Duration::ZERO,
            Instant::now()
        ));
    }
}
