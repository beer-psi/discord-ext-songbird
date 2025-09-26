use async_trait::async_trait;
use pyo3::{prelude::*, Py, PyAny, Python};
use songbird::{tracks::PlayMode, Event, EventContext, EventHandler};

use crate::error::{PyPlayError, SongbirdError};

#[pyclass(module = "discord.ext.songbird._native.events.track", frozen)]
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

pub struct PythonTrackEventHandler {
    pub callback: Py<PyAny>,
}

#[async_trait]
impl EventHandler for PythonTrackEventHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let songbird::EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                // this shouldn't be terribly expensive, it's just a call to
                // Py_INCREF
                let callback = Python::attach(|py| self.callback.clone_ref(py));
                let handle_uuid = handle.uuid().clone();

                match state.playing {
                    PlayMode::Errored(_) => {
                        let msg = format!("{:?}", state.playing);

                        tokio::task::spawn_blocking(move || {
                            let py_err: PyErr = PyPlayError::new_err(msg);

                            Python::attach(|py| {
                                let _ = callback.call1(py, (handle_uuid, py_err));
                                callback.drop_ref(py);
                            });
                        });
                    }
                    _ => {
                        tokio::task::spawn_blocking(move || {
                            Python::attach(|py| {
                                let _ = callback.call1(py, (handle_uuid,));
                                callback.drop_ref(py);
                            });
                        });
                    }
                }
            }
        }

        None
    }
}
