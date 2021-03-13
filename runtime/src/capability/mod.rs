mod accelerometer;
mod image;
mod random;
mod sound;

pub use self::{
    accelerometer::Accelerometer, random::Random, sound::Sound, image::Image,
};

use anyhow::Error;
use runic_types::{Type, Value};
use std::fmt::Debug;

pub trait Capability: Send + Debug + 'static {
    /// Generate the desired input, writing it to the provided buffer and
    /// returning the number of bytes written.
    fn generate(&mut self, buffer: &mut [u8]) -> Result<usize, Error>;

    fn set_parameter(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), ParameterError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ParameterError {
    #[error("The \"{}\" parameter isn't supported", name)]
    UnsupportedParameter { name: String },
    #[error("{:?} is an invalid value for \"{}\"", value, name)]
    InvalidValue {
        name: String,
        value: Value,
        #[source]
        reason: Error,
    },
    #[error("Expected a {:?} but found {:?}", expected, actual)]
    IncorrectType { expected: Type, actual: Type },
}

impl ParameterError {
    pub fn unsupported(name: impl Into<String>) -> Self {
        ParameterError::UnsupportedParameter { name: name.into() }
    }
}
