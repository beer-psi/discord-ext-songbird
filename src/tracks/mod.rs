pub mod handle;
pub mod looping;
pub mod mode;

use pyo3::{intern, prelude::*, PyTraverseError, PyVisit};
use songbird::tracks::Track as SongbirdTrack;
use uuid::Uuid;

use crate::{
    input::sources::base::{AudioSource, SongbirdSource},
    tracks::{looping::LoopState, mode::PlayMode},
};

/// Initial state for audio playback.
#[pyclass(module = "discord.ext.songbird._native.tracks")]
#[derive(Debug)]
pub struct Track {
    #[pyo3(get, set)]
    pub source: Py<AudioSource>,

    #[pyo3(get, set)]
    pub playing: PlayMode,

    #[pyo3(get, set)]
    pub volume: f32,

    #[pyo3(get, set)]
    pub loop_state: LoopState,

    #[pyo3(get, set)]
    pub uuid: Uuid,
}

#[pymethods]
impl Track {
    #[new]
    #[pyo3(signature = (source, playing = PlayMode::Play, volume = 1.0, loop_state = LoopState::_Finite(0), uuid = None))]
    pub fn new(
        source: Py<AudioSource>,
        playing: PlayMode,
        volume: f32,
        loop_state: LoopState,
        uuid: Option<Uuid>,
    ) -> Self {
        Self {
            source,
            playing,
            volume,
            loop_state,
            uuid: uuid.unwrap_or_else(|| Uuid::new_v4()),
        }
    }

    fn play<'py>(mut slf: PyRefMut<'py, Self>) -> PyRefMut<'py, Self> {
        slf.playing = PlayMode::Play;
        slf
    }

    fn pause<'py>(mut slf: PyRefMut<'py, Self>) -> PyRefMut<'py, Self> {
        slf.playing = PlayMode::Pause;
        slf
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.source)?;
        Ok(())
    }
}

impl Track {
    pub fn into_songbird_track(&self, py: Python<'_>) -> Result<SongbirdTrack, PyErr> {
        let input = self
            .source
            .call_method0(py, intern!(py, "_get_songbird_source"))?
            .cast_bound::<SongbirdSource>(py)?
            .get()
            .0
            .input()?;
        let mut track = SongbirdTrack::new_with_uuid(input, self.uuid)
            .volume(self.volume)
            .loops(self.loop_state.into());

        track.playing = self.playing.into();

        Ok(track)
    }
}
