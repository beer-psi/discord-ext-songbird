use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use pyo3::prelude::*;
use songbird::input::File as SongbirdFile;

use crate::{
    error::{SongbirdError, SongbirdResult},
    input::sources::base::{AudioSource, IntoSongbirdInput, SongbirdSource},
};

#[pyclass(
    from_py_object,
    extends = AudioSource,
    module = "discord.ext.songbird._native.input.sources"
)]
#[derive(Clone, Debug)]
pub struct File {
    inner: Arc<RwLock<Option<SongbirdFile<PathBuf>>>>,
}

#[pymethods]
impl File {
    #[new]
    fn new(path: PathBuf) -> (Self, AudioSource) {
        (
            Self {
                inner: Arc::new(RwLock::new(Some(SongbirdFile::new(path)))),
            },
            AudioSource::new(),
        )
    }

    fn _get_songbird_source(&self) -> SongbirdResult<SongbirdSource> {
        Ok(SongbirdSource(Box::new(self.clone())))
    }
}

impl IntoSongbirdInput for File {
    fn input(&self) -> SongbirdResult<songbird::input::Input> {
        let songbird_file = self.inner.write().unwrap().take();

        if let Some(songbird_file) = songbird_file {
            Ok(songbird_file.into())
        } else {
            Err(SongbirdError::SourceConsumed)
        }
    }
}
