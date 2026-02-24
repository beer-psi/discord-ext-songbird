use async_trait::async_trait;
use pyo3::{prelude::*, sync::PyOnceLock, types::PyFunction, Py, PyAny, Python};
use pyo3_async_runtimes::TaskLocals;
use songbird::{tracks::PlayMode, Event, EventContext, EventHandler};
use tracing::error;

use crate::error::{PyPlayError, SongbirdError};

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

pub struct PythonTrackEventHandler {
    pub callback: Py<PyAny>,
    pub task_locals: TaskLocals,
}

macro_rules! dispatch_callback {
    ($callback:expr, $task_locals:expr, $args:expr) => {
        tokio::task::spawn_blocking(move || {
            let result = Python::attach(|py| -> PyResult<_> {
                let result = match $callback.call1(py, $args) {
                    Ok(r) => {
                        $callback.drop_ref(py);
                        r.into_bound(py)
                    }
                    Err(e) => {
                        $callback.drop_ref(py);
                        return Err(e);
                    }
                };
                let future = if is_awaitable(&result)? {
                    Ok(Some(pyo3_async_runtimes::into_future_with_locals(
                        &$task_locals,
                        result,
                    )?))
                } else {
                    Ok(None)
                };

                future
            });

            match result {
                Ok(Some(future)) => {
                    tokio::task::spawn(future);
                },
                Ok(None) => {},
                Err(e) => {
                    error!(error = ?e, "could not dispatch event callback");
                }
            }
        });
    };
}

#[async_trait]
impl EventHandler for PythonTrackEventHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let songbird::EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                // this shouldn't be terribly expensive, it's just a call to
                // Py_INCREF
                let callback = Python::attach(|py| self.callback.clone_ref(py));
                let task_locals = self.task_locals.clone();
                let handle_uuid = handle.uuid().clone();

                match state.playing {
                    PlayMode::Errored(_) => {
                        let msg = format!("{:?}", state.playing);

                        dispatch_callback!(
                            callback,
                            task_locals,
                            (handle_uuid, PyPlayError::new_err(msg))
                        );
                    }
                    _ => {
                        dispatch_callback!(callback, task_locals, (handle_uuid,));
                    }
                }
            }
        }

        None
    }
}

fn is_awaitable(object: &Bound<'_, PyAny>) -> PyResult<bool> {
    static INSPECT_MODULE: PyOnceLock<Py<PyFunction>> = PyOnceLock::new();
    let py = object.py();

    INSPECT_MODULE
        .import(py, "inspect", "isawaitable")?
        .call1((object,))?
        .extract()
}
