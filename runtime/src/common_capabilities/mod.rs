mod accelerometer;
mod image;
mod random;
mod sound;
mod raw;

use crate::ParameterError;

pub use self::{
    accelerometer::Accelerometer, random::Random, sound::Sound, image::Image,
    raw::Raw,
};

use anyhow::Error;
use runic_types::Value;
use std::convert::{TryFrom, TryInto};

fn try_from_int_value<T>(value: Value) -> Result<T, ParameterError>
where
    T: TryFrom<i32>,
    T::Error: Into<Error>,
{
    let integer: i32 = value
        .try_into()
        .map_err(|e| ParameterError::IncorrectType(e))?;

    T::try_from(integer).map_err(|e| ParameterError::InvalidValue {
        value,
        reason: e.into(),
    })
}
