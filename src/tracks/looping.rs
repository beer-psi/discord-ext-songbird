use pyo3::prelude::*;
use songbird::tracks::LoopState as SongbirdLoopState;

#[pyclass(module = "discord.ext.songbird._native.tracks", frozen, from_py_object)]
#[derive(Clone, Copy, Debug)]
pub enum LoopState {
    _Infinite(),
    _Finite(usize),
}

#[pymethods]
impl LoopState {
    #[staticmethod]
    pub fn infinite() -> Self {
        LoopState::_Infinite()
    }

    #[staticmethod]
    #[pyo3(signature = (count = 0))]
    pub fn finite(count: usize) -> Self {
        LoopState::_Finite(count)
    }
}

impl From<LoopState> for SongbirdLoopState {
    fn from(value: LoopState) -> Self {
        match value {
            LoopState::_Infinite() => SongbirdLoopState::Infinite,
            LoopState::_Finite(count) => SongbirdLoopState::Finite(count),
        }
    }
}
