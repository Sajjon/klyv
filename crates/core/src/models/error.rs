use derive_more::Debug;

pub const BINARY_NAME: &str = "klyv";

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("An error occurred {underlying}")]
    Generic { underlying: String },
}

impl Error {
    pub fn bail(msg: impl AsRef<str>) -> Self {
        Self::Generic {
            underlying: msg.as_ref().to_owned(),
        }
    }
    pub fn from(error: impl std::error::Error) -> Self {
        Self::Generic {
            underlying: error.to_string(),
        }
    }
}
