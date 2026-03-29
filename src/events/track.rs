use pyo3::prelude::*;

use crate::error::SongbirdError;

#[pyclass(module = "discord.ext.songbird._native.events", frozen, from_py_object)]
#[derive(Clone, Copy, Debug)]
pub enum TrackEvent {
    Play,
    Pause,
    End,
    Loop,
    Preparing,
    Playable,
    Error,
}

impl TryFrom<songbird::events::TrackEvent> for TrackEvent {
    type Error = SongbirdError;

    fn try_from(value: songbird::events::TrackEvent) -> Result<Self, SongbirdError> {
        match value {
            songbird::events::TrackEvent::Play => Ok(TrackEvent::Play),
            songbird::events::TrackEvent::Pause => Ok(TrackEvent::Pause),
            songbird::events::TrackEvent::End => Ok(TrackEvent::End),
            songbird::events::TrackEvent::Loop => Ok(TrackEvent::Loop),
            songbird::events::TrackEvent::Preparing => Ok(TrackEvent::Preparing),
            songbird::events::TrackEvent::Playable => Ok(TrackEvent::Playable),
            songbird::events::TrackEvent::Error => Ok(TrackEvent::Error),
            _ => Err(SongbirdError::UnknownTrackEvent(value)),
        }
    }
}

impl From<TrackEvent> for songbird::events::TrackEvent {
    fn from(value: TrackEvent) -> Self {
        match value {
            TrackEvent::Play => songbird::events::TrackEvent::Play,
            TrackEvent::Pause => songbird::events::TrackEvent::Pause,
            TrackEvent::End => songbird::events::TrackEvent::End,
            TrackEvent::Loop => songbird::events::TrackEvent::Loop,
            TrackEvent::Preparing => songbird::events::TrackEvent::Preparing,
            TrackEvent::Playable => songbird::events::TrackEvent::Playable,
            TrackEvent::Error => songbird::events::TrackEvent::Error,
        }
    }
}
