#![expect(deprecated, reason = "Added deprecated attribute")]
use pyo3::prelude::*;

#[pyclass(frozen, from_py_object, module = "discord.ext.songbird._native")]
#[derive(Clone, Copy, Debug)]
pub enum Bitrate {
    _Bits(i32),
    _Max(),
    _Auto(),
}

#[pymethods]
impl Bitrate {
    #[staticmethod]
    #[deprecated(since = "0.2.0", note = "Use `Bitrate::bits`")]
    pub fn bits_per_second(bitrate: i32) -> Self {
        Bitrate::_Bits(bitrate)
    }

    #[staticmethod]
    pub fn bits(bitrate: i32) -> Self {
        Bitrate::_Bits(bitrate)
    }

    #[staticmethod]
    pub fn max() -> Self {
        Bitrate::_Max()
    }

    #[staticmethod]
    pub fn auto() -> Self {
        Bitrate::_Auto()
    }
}

impl From<Bitrate> for songbird::driver::Bitrate {
    fn from(value: Bitrate) -> Self {
        match value {
            Bitrate::_Bits(bitrate) => songbird::driver::Bitrate::Bits(bitrate),
            Bitrate::_Max() => songbird::driver::Bitrate::Max,
            Bitrate::_Auto() => songbird::driver::Bitrate::Auto,
        }
    }
}
