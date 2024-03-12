use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Deserialise(#[from] toml::de::Error),
    #[error(transparent)]
    Serialise(#[from] toml::ser::Error),
}
