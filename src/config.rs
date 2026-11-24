use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
#[error("invalid value for {0}")]
pub struct InvalidConfig(&'static str);

pub struct Config {
    pub address: SocketAddr,
    pub bin: PathBuf,
    pub key: Option<String>,
    pub models_file: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
    pub timeout: Duration,
    pub pending: usize,
    pub body_bytes: usize,
    pub prompt_bytes: usize,
    pub output_bytes: usize,
}

impl Config {
    pub fn read() -> Result<Self, InvalidConfig> {
        let host = match value("AGY_HOST")? {
            Some(host) => host
                .parse::<IpAddr>()
                .ok()
                .filter(IpAddr::is_loopback)
                .ok_or(InvalidConfig("AGY_HOST"))?,
            None => Ipv4Addr::LOCALHOST.into(),
        };
        let port = number("AGY_PORT", 8766, u16::MAX as usize)? as u16;
        let key = value("AGY_API_KEY")?;
        if let Some(key) = &key {
            if key.is_empty()
                || key.len() > 1024
                || !key.bytes().all(|byte| byte.is_ascii_graphic())
            {
                return Err(InvalidConfig("AGY_API_KEY"));
            }
        }

        Ok(Self {
            address: SocketAddr::new(host, port),
            bin: path("AGY_BIN")?.unwrap_or_else(|| PathBuf::from("agy")),
            key,
            models_file: path("AGY_MODELS_FILE")?,
            workspace: path("AGY_WORKSPACE")?,
            timeout: Duration::from_secs(number("AGY_TIMEOUT_SECS", 120, 1800)? as u64),
            pending: number("AGY_MAX_PENDING", 8, 64)?,
            body_bytes: number("AGY_MAX_BODY_BYTES", 262_144, 4_194_304)?,
            prompt_bytes: number("AGY_MAX_PROMPT_BYTES", 16_384, 24_576)?,
            output_bytes: number("AGY_MAX_OUTPUT_BYTES", 1_048_576, 8_388_608)?,
        })
    }
}

fn value(name: &'static str) -> Result<Option<String>, InvalidConfig> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(InvalidConfig(name)),
    }
}

fn path(name: &'static str) -> Result<Option<PathBuf>, InvalidConfig> {
    match env::var_os(name) {
        Some(value) if value.is_empty() => Err(InvalidConfig(name)),
        Some(value) => Ok(Some(value.into())),
        None => Ok(None),
    }
}

fn number(name: &'static str, default: usize, max: usize) -> Result<usize, InvalidConfig> {
    match value(name)? {
        None => Ok(default),
        Some(value) => parse_number(&value, max).ok_or(InvalidConfig(name)),
    }
}

fn parse_number(value: &str, max: usize) -> Option<usize> {
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value
        .parse()
        .ok()
        .filter(|number| (1..=max).contains(number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_port_numbers() {
        assert_eq!(parse_number("00080", 65_535), Some(80));
        assert_eq!(parse_number("65535", 65_535), Some(65_535));
        for value in ["", "0", "65536", "٣", "+8", "8.0", " 80", "80\n"] {
            assert_eq!(parse_number(value, 65_535), None, "{value:?}");
        }
    }
}
