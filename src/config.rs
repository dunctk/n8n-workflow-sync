use std::env;
use url::Url;

#[derive(Debug, Clone)]
pub struct N8nConfig {
    pub api_key: String,
    pub host: Url,
}

impl N8nConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = env::var("N8N_API_KEY")?;
        let mut host = env::var("N8N_HOST")?;
        host = host.trim_end_matches('/').to_string();
        if host.ends_with("/v1") {
            host = host.trim_end_matches("/v1").to_string();
        }
        host = format!("{}/", host);
        let host = Url::parse(&host)?;
        Ok(Self { api_key, host })
    }

    pub fn endpoint(&self, path: &str) -> Url {
        self.host
            .join(&format!("v1/{}", path.trim_start_matches('/')))
            .expect("valid base url")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use temp_env::with_vars;

    #[test]
    #[serial]
    fn reads_from_env() {
        with_vars(
            [
                ("N8N_API_KEY", Some("test-key")),
                ("N8N_HOST", Some("http://localhost")),
            ],
            || {
                let cfg = N8nConfig::from_env().unwrap();
                assert_eq!(cfg.api_key, "test-key");
                assert_eq!(cfg.host.as_str(), "http://localhost/");
            },
        );
    }
}
