use std::time::Duration;

use nonmax::NonMaxU32;
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
};
use pyo3_async_runtimes::tokio::future_into_py;
use songbird::{
    events::TrackEvent as SongbirdTrackEvent, tracks::TrackHandle as SongbirdTrackHandle,
};
use uuid::Uuid;

use crate::{
    error::{SongbirdError, SongbirdResult},
    events::{track::TrackEvent, PythonEventHandler},
};

#[pyclass(module = "discord.ext.songbird._native.tracks", frozen)]
pub struct TrackHandle {
    inner: SongbirdTrackHandle,
}

impl TrackHandle {
    pub fn new(handle: SongbirdTrackHandle) -> Self {
        Self { inner: handle }
    }
}

#[pymethods]
impl TrackHandle {
    pub fn play(&self) -> SongbirdResult<()> {
        self.inner.play()?;
        Ok(())
    }

    pub fn pause(&self) -> SongbirdResult<()> {
        self.inner.pause()?;
        Ok(())
    }

    pub fn stop(&self) -> SongbirdResult<()> {
        self.inner.stop()?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f32) -> SongbirdResult<()> {
        self.inner.set_volume(volume)?;
        Ok(())
    }

    pub fn make_playable<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let cb = self.inner.make_playable();

        future_into_py(py, async {
            cb.result_async()
                .await
                .map_err(|e| SongbirdError::ControlError(e))?;

            Ok(())
        })
    }

    pub fn seek<'py>(&self, py: Python<'py>, position: Duration) -> PyResult<Bound<'py, PyAny>> {
        let cb = self.inner.seek(position);

        future_into_py(py, async {
            cb.result_async()
                .await
                .map_err(|e| SongbirdError::ControlError(e))?;

            Ok(())
        })
    }

    /// Attach an event handler to an audio track. This method requires an active event loop.
    pub fn add_event(&self, py: Python, event: TrackEvent, callback: Py<PyAny>) -> PyResult<()> {
        if !callback.bind(py).is_callable() {
            return Err(PyTypeError::new_err("event handler must be callable"));
        }

        let task_locals = pyo3_async_runtimes::tokio::get_current_locals(py)?;

        py.detach(move || {
            let songbird_event: SongbirdTrackEvent = event.into();

            self.inner
                .add_event(
                    songbird_event.into(),
                    PythonEventHandler {
                        callback,
                        task_locals,
                    },
                )
                .map_err(|e| SongbirdError::ControlError(e))?;

            Ok(())
        })
    }

    // action
    // get_info

    pub fn enable_loop(&self) -> SongbirdResult<()> {
        self.inner.enable_loop()?;
        Ok(())
    }

    pub fn disable_loop(&self) -> SongbirdResult<()> {
        self.inner.disable_loop()?;
        Ok(())
    }

    pub fn loop_for(&self, count: u32) -> PyResult<()> {
        let Some(count) = NonMaxU32::new(count) else {
            return Err(PyValueError::new_err("invalid u32::MAX value"));
        };

        self.inner
            .loop_for(count)
            .map_err(|e| SongbirdError::ControlError(e))?;
        Ok(())
    }

    #[getter]
    pub fn uuid(&self) -> Uuid {
        self.inner.uuid()
    }

    // data
}
