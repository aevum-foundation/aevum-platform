use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub environment: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("AEVUM_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("AEVUM_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            environment: env::var("AEVUM_ENV").unwrap_or_else(|_| "development".to_string()),
        }
    }
}
