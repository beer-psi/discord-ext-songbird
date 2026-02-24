use pyo3::prelude::*;
use songbird::tracks::PlayMode as SongbirdPlayMode;

#[pyclass(module = "discord.ext.songbird._native.tracks", from_py_object)]
#[derive(Clone, Copy, Debug)]
pub enum PlayMode {
    Play,
    Pause,
}

impl From<PlayMode> for SongbirdPlayMode {
    fn from(value: PlayMode) -> Self {
        match value {
            PlayMode::Play => SongbirdPlayMode::Play,
            PlayMode::Pause => SongbirdPlayMode::Pause,
        }
    }
}
