mod error;

pub mod prelude {

    pub use crate::error::*;

    pub use bon::Builder;
    pub use getset::Getters;
    pub use log::*;
}
