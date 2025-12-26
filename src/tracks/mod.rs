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

/// Immutable initial state for audio playback.
#[pyclass(frozen, module = "discord.ext.songbird._native.tracks")]
#[derive(Debug)]
pub struct Track {
    #[pyo3(get)]
    pub source: Py<AudioSource>,

    #[pyo3(get)]
    pub playing: PlayMode,

    #[pyo3(get)]
    pub volume: f32,

    #[pyo3(get)]
    pub loop_state: LoopState,

    #[pyo3(get)]
    pub uuid: Uuid,
}

#[pymethods]
impl Track {
    #[new]
    pub fn new(
        source: Py<AudioSource>,
        playing: PlayMode,
        volume: f32,
        loop_state: LoopState,
        uuid: Uuid,
    ) -> Self {
        Self {
            source,
            playing,
            volume,
            loop_state,
            uuid,
        }
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.source)?;
        Ok(())
    }
}

impl Track {
    pub fn into_songbird_track(&self) -> Result<SongbirdTrack, PyErr> {
        let input = Python::attach(|py| {
            let songbird_source = self
                .source
                .call_method0(py, intern!(py, "_get_songbird_source"))?;
            let songbird_source = songbird_source.cast_bound::<SongbirdSource>(py)?;

            PyResult::<songbird::input::Input>::Ok(songbird_source.get().0.input()?)
        })?;
        let mut track = SongbirdTrack::new_with_uuid(input, self.uuid)
            .volume(self.volume)
            .loops(self.loop_state.into());

        track.playing = self.playing.into();

        Ok(track)
    }
}
