use nonmax::NonMaxU32;
use pyo3::{exceptions::PyValueError, prelude::*};
use songbird::tracks::LoopState as SongbirdLoopState;

#[pyclass(module = "discord.ext.songbird._native.tracks", frozen, from_py_object)]
#[derive(Clone, Copy, Debug)]
pub enum LoopState {
    _Infinite(),
    _Finite(u32),
}

#[pymethods]
impl LoopState {
    #[staticmethod]
    pub fn infinite() -> Self {
        LoopState::_Infinite()
    }

    #[staticmethod]
    #[pyo3(signature = (count = 0))]
    pub fn finite(count: u32) -> PyResult<Self> {
        let Some(count) = NonMaxU32::new(count) else {
            return Err(PyValueError::new_err("invalid u32::MAX value"));
        };

        Ok(LoopState::_Finite(count.get()))
    }
}

impl From<LoopState> for SongbirdLoopState {
    fn from(value: LoopState) -> Self {
        match value {
            LoopState::_Infinite() => SongbirdLoopState::Infinite,
            LoopState::_Finite(count) => {
                // SAFETY: Has been checked by LoopState::finite above, which is the only
                // way to construct a LoopState::_Finite.
                SongbirdLoopState::Finite(unsafe { NonMaxU32::new_unchecked(count) })
            }
        }
    }
}
