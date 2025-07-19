use derive_more::Debug;

pub const BINARY_NAME: &str = "klyv";

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("An error occurred")]
    Generic,
}
