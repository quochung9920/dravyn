use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

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
            let endpoint = format!("{}://{}:{}", proxy.scheme.as_str(), proxy.host, proxy.port);
            let addresses = match (proxy.host.as_str(), proxy.port).to_socket_addrs() {
                Ok(addresses) => addresses.take(8).collect::<Vec<_>>(),
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
            let reachable = addresses
                .iter()
                .any(|address| TcpStream::connect_timeout(address, timeout).is_ok());
            let latency_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            NetworkProbeResult {
                mode: "proxy".to_owned(),
                endpoint: Some(endpoint),
                valid: true,
                reachable: Some(reachable),
                latency_ms: Some(latency_ms),
                message: if reachable {
                    "Proxy endpoint accepted a TCP connection. This proves endpoint reachability only; browser-side IP/DNS/WebRTC exposure still requires verification.".to_owned()
                } else {
                    "Proxy endpoint could not be reached within the configured preflight timeout.".to_owned()
                },
            }
        }
    }
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
}
