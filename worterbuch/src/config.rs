use std::{env, net::IpAddr, time::Duration};
use worterbuch_common::{
    error::{ConfigIntContext, ConfigResult},
    Path,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub tls: bool,
    pub bind_addr: IpAddr,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WsEndpoint {
    pub endpoint: Endpoint,
    pub public_addr: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub ws_endpoint: Option<WsEndpoint>,
    pub tcp_endpoint: Option<Endpoint>,
    pub use_persistence: bool,
    pub persistence_interval: Duration,
    pub data_dir: Path,
    pub single_threaded: bool,
    pub web_root_path: Option<String>,
    pub keepalive_timeout: Duration,
    pub send_timeout: Duration,
    pub channel_buffer_size: usize,
    pub extended_monitoring: bool,
}

impl Config {
    pub fn load_env(&mut self) -> ConfigResult<()> {
        self.load_env_with_prefix("WORTERBUCH")
    }

    pub fn load_env_with_prefix(&mut self, prefix: &str) -> ConfigResult<()> {
        if let Ok(val) = env::var(prefix.to_owned() + "_WS_TLS") {
            if let Some(ep) = &mut self.ws_endpoint {
                ep.endpoint.tls = val.to_lowercase() == "true" || val == "1";
            }
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_WS_SERVER_PORT") {
            if let Some(ep) = &mut self.ws_endpoint {
                ep.endpoint.port = val.parse().as_port()?;
            }
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_WS_BIND_ADDRESS") {
            if let Some(ep) = &mut self.ws_endpoint {
                ep.endpoint.bind_addr = val.parse()?;
            }
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_PUBLIC_ADDRESS") {
            if let Some(ep) = &mut self.ws_endpoint {
                ep.public_addr = val;
            }
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_TCP_SERVER_PORT") {
            if let Some(ep) = &mut self.tcp_endpoint {
                ep.port = val.parse().as_port()?;
            }
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_TCP_BIND_ADDRESS") {
            if let Some(ep) = &mut self.tcp_endpoint {
                ep.bind_addr = val.parse()?;
            }
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_USE_PERSISTENCE") {
            self.use_persistence = val.to_lowercase() == "true";
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_PERSISTENCE_INTERVAL") {
            let secs = val.parse().as_interval()?;
            self.persistence_interval = Duration::from_secs(secs);
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_DATA_DIR") {
            self.data_dir = val;
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_SINGLE_THREADED") {
            self.single_threaded = val.to_lowercase() == "true";
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_WEBROOT_PATH") {
            self.web_root_path = Some(val);
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_KEEPALIVE_TIMEOUT") {
            let secs = val.parse().as_interval()?;
            self.keepalive_timeout = Duration::from_secs(secs);
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_SEND_TIMEOUT") {
            let secs = val.parse().as_interval()?;
            self.send_timeout = Duration::from_secs(secs);
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_CHANNEL_BUFFER_SIZE") {
            let size = val.parse().as_interval()?;
            self.channel_buffer_size = size;
        }

        if let Ok(val) = env::var(prefix.to_owned() + "_EXTENDED_MONITORING") {
            let enabled = val.to_lowercase();
            let enabled = enabled.trim();
            self.extended_monitoring = enabled == "true" || enabled == "1";
        }

        Ok(())
    }

    pub fn new() -> ConfigResult<Self> {
        let mut config = Config::default();
        config.load_env()?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ws_endpoint: Some(WsEndpoint {
                endpoint: Endpoint {
                    tls: false,
                    bind_addr: [127, 0, 0, 1].into(),
                    port: 8080,
                },
                public_addr: "localhost".to_owned(),
            }),
            tcp_endpoint: Some(Endpoint {
                tls: false,
                bind_addr: [127, 0, 0, 1].into(),
                port: 8081,
            }),
            use_persistence: false,
            persistence_interval: Duration::from_secs(30),
            data_dir: "./data".into(),
            single_threaded: false,
            web_root_path: None,
            keepalive_timeout: Duration::from_secs(5),
            send_timeout: Duration::from_secs(5),
            channel_buffer_size: 1_000,
            extended_monitoring: true,
        }
    }
}
