use pyo3::{exceptions::PyNotImplementedError, prelude::*};
use songbird::input::Input;

use crate::error::SongbirdResult;

#[pyclass(subclass, module = "discord.ext.songbird._native.input.sources")]
#[derive(Clone, Debug)]
pub struct AudioSource {}

impl AudioSource {
    pub fn new() -> Self {
        Self {}
    }
}

#[pymethods]
impl AudioSource {
    pub fn _get_songbird_source(&self) -> PyResult<SongbirdSource> {
        Err(PyNotImplementedError::new_err(()))
    }
}

#[pyclass(frozen)]
pub struct SongbirdSource(pub Box<dyn IntoSongbirdInput>);

pub trait IntoSongbirdInput: Send + Sync {
    fn input(&self) -> SongbirdResult<Input>;
}
