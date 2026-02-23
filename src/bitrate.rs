use pyo3::prelude::*;

#[pyclass(frozen, from_py_object, module = "discord.ext.songbird._native")]
#[derive(Clone, Copy, Debug)]
pub enum Bitrate {
    _BitsPerSecond(i32),
    _Max(),
    _Auto(),
}

#[pymethods]
impl Bitrate {
    #[staticmethod]
    pub fn bits_per_second(bitrate: i32) -> Self {
        Bitrate::_BitsPerSecond(bitrate)
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
            Bitrate::_BitsPerSecond(bitrate) => songbird::driver::Bitrate::BitsPerSecond(bitrate),
            Bitrate::_Max() => songbird::driver::Bitrate::Max,
            Bitrate::_Auto() => songbird::driver::Bitrate::Auto,
        }
    }
}
