use async_trait::async_trait;
use pyo3::{
    sync::PyOnceLock,
    types::{PyAnyMethods, PyFunction},
    Bound, Py, PyAny, PyResult, Python,
};
use pyo3_async_runtimes::TaskLocals;
use songbird::{tracks::PlayMode, Event, EventContext, EventHandler};

use crate::error::PyPlayError;

pub mod track;

/// Dispatches a Python callback inside a blocking Tokio worker using the
/// provided [`TaskLocals`] and arguments. If the callback returns an awaitable,
/// an additional Tokio worker is spawned to poll the future to completion.
///
/// [`TaskLocals`]: `pyo3_async_runtimes::TaskLocals`
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
                    tracing::error!(error = ?e, "could not dispatch event callback");
                }
            }
        });
    };
}

fn is_awaitable(object: &Bound<'_, PyAny>) -> PyResult<bool> {
    static INSPECT_MODULE: PyOnceLock<Py<PyFunction>> = PyOnceLock::new();
    let py = object.py();

    INSPECT_MODULE
        .import(py, "inspect", "isawaitable")?
        .call1((object,))?
        .extract()
}

pub struct PythonEventHandler {
    pub callback: Py<PyAny>,
    pub task_locals: TaskLocals,
}

#[async_trait]
impl EventHandler for PythonEventHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            songbird::EventContext::Track(track_list) => {
                for (state, handle) in *track_list {
                    // this shouldn't be terribly expensive, it's just a call to
                    // Py_INCREF
                    let callback = Python::attach(|py| self.callback.clone_ref(py));
                    let task_locals = self.task_locals.clone();
                    let handle_uuid = handle.uuid().clone();

                    match &state.playing {
                        PlayMode::Errored(e) => {
                            let msg = format!("{:?}", e);

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
            // EventContext::SpeakingStateUpdate(speaking) => {
            //     let callback = Python::attach(|py| self.callback.clone_ref(py));
            //     let task_locals = self.task_locals.clone();
            // }
            // EventContext::ClientDisconnect(client_disconnect) => todo!(),
            // EventContext::DriverConnect(connect_data) => todo!(),
            // EventContext::DriverReconnect(connect_data) => todo!(),
            // EventContext::DriverDisconnect(disconnect_data) => todo!(),
            _ => {}
        }

        None
    }
}
