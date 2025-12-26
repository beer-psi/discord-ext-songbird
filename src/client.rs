use std::{num::NonZeroU64, sync::Arc, time::Duration};

use pyo3::{exceptions::PyTypeError, intern, prelude::*};
use pyo3_async_runtimes::tokio::future_into_py;
use songbird::id::ChannelId;

use crate::{
    bitrate::Bitrate,
    config::Config,
    connection::{DiscordPyVoiceUpdate, VoiceConnection},
    input::sources::base::{AudioSource, SongbirdSource},
    tracks::{handle::TrackHandle, Track},
};

#[pyclass(module = "discord.ext.songbird._native")]
pub struct SongbirdClient {
    #[pyo3(get)]
    pub config: Config,

    connection: Arc<VoiceConnection>,
}

#[pymethods]
impl SongbirdClient {
    #[staticmethod]
    pub fn new<'py>(
        py: Python<'py>,
        config: Config,
        user_id: NonZeroU64,
        guild_id: NonZeroU64,
        channel_id: NonZeroU64,
        update_voice_state_hook: Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if !update_voice_state_hook.bind(py).is_callable() {
            return Err(PyTypeError::new_err(
                "expected update_voice_state_hook to be callable",
            ));
        }

        future_into_py(py, async move {
            // we have to instantiate the connection in here since songbird's call
            // wants to start the driver threads using Tokio
            let connection = Arc::new(VoiceConnection::new(
                user_id,
                guild_id,
                channel_id,
                DiscordPyVoiceUpdate {
                    update_voice_state_hook,
                },
            ));

            Ok(Self { config, connection })
        })
    }

    /// Updates the voice server data.
    pub fn update_server<'py>(
        &self,
        py: Python<'py>,
        endpoint: String,
        token: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.update_server(endpoint, token).await?;
            Ok(())
        })
    }

    /// Updates the internal voice state of the current user.
    pub fn update_state<'py>(
        &self,
        py: Python<'py>,
        session_id: String,
        channel_id: Option<NonZeroU64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            let channel_id: Option<ChannelId> = channel_id.map(|c| c.into());

            conn.update_state(session_id, channel_id).await?;
            Ok(())
        })
    }

    /// Connect to the voice channel specified at creation.
    pub fn connect<'py>(
        &self,
        py: Python<'py>,
        timeout: Duration,
        reconnect: bool,
        self_deaf: bool,
        self_mute: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();
        let config = self.config.clone();

        future_into_py(py, async move {
            conn.connect(config, timeout, reconnect, self_deaf, self_mute)
                .await?;
            Ok(())
        })
    }

    /// Terminate the connection. The connection is no longer usable.
    pub fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.disconnect().await?;
            Ok(())
        })
    }

    /// Moves the current user to a different voice channel.
    pub fn move_to<'py>(
        &self,
        py: Python<'py>,
        channel_id: Option<NonZeroU64>,
        timeout: Option<Duration>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.move_to(channel_id, timeout).await?;
            Ok(())
        })
    }

    /// Indicates if the client is connected to a voice channel.
    pub fn is_connected<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move { Ok(conn.is_connected().await?) })
    }

    pub fn mute<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.mute(true).await?;
            Ok(())
        })
    }

    pub fn unmute<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.mute(false).await?;
            Ok(())
        })
    }

    pub fn deafen<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.deafen(true).await?;
            Ok(())
        })
    }

    pub fn undeafen<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.deafen(false).await?;
            Ok(())
        })
    }

    /// Plays a track.
    ///
    /// Since Track is immutable, this method also supports any class with a
    /// :meth:`into_track` method that returns a Track, in order to customize
    /// the track before playing.
    ///
    /// The underlying audio source is consumed. If it is reused, a :class:`SourceConsumed`
    /// exception is raised.
    pub fn play<'py>(&self, py: Python<'py>, track: Py<PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();
        let songbird_track = convert_track_to_songbird_track(py, track)?;

        future_into_py(py, async move {
            let handle = conn.play(songbird_track).await?;

            Ok(TrackHandle::new(handle))
        })
    }

    /// Similar to :meth:`play`, except that it stops all other sources attached to this connection.
    pub fn play_only<'py>(&self, py: Python<'py>, track: Py<PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();
        let songbird_track = convert_track_to_songbird_track(py, track)?;

        future_into_py(py, async move {
            let handle = conn.play_only(songbird_track).await?;

            Ok(TrackHandle::new(handle))
        })
    }

    /// Plays the given audio source.
    ///
    /// The audio source is consumed. If it is reused, a :class:`SourceConsumed`
    /// exception is raised.
    pub fn play_input<'py>(
        &self,
        py: Python<'py>,
        source: Py<AudioSource>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let input = convert_audio_source_to_songbird_input(py, source)?;
        let conn = self.connection.clone();

        future_into_py(py, async move {
            let handle = conn.play_input(input).await?;

            Ok(TrackHandle::new(handle))
        })
    }

    /// Similar to :meth:`play_input`, except that it stops all other sources attached to this connection.
    pub fn play_only_input<'py>(
        &self,
        py: Python<'py>,
        source: Py<AudioSource>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let input = convert_audio_source_to_songbird_input(py, source)?;
        let conn = self.connection.clone();

        future_into_py(py, async move {
            let handle = conn.play_only_input(input).await?;

            Ok(TrackHandle::new(handle))
        })
    }

    pub fn set_bitrate<'py>(
        &self,
        py: Python<'py>,
        bitrate: Bitrate,
    ) -> PyResult<Bound<'py, PyAny>> {
        let conn = self.connection.clone();

        future_into_py(py, async move {
            conn.set_bitrate(bitrate.into()).await?;

            Ok(())
        })
    }
}

fn convert_track_to_songbird_track(
    py: Python,
    track: Py<PyAny>,
) -> PyResult<songbird::tracks::Track> {
    if let Ok(track) = track.cast_bound::<Track>(py) {
        track.get().into_songbird_track()
    } else {
        let track = track.call_method0(py, intern!(py, "into_track"))?;
        let track = track.cast_bound::<Track>(py)?;

        track.get().into_songbird_track()
    }
}

fn convert_audio_source_to_songbird_input(
    py: Python,
    source: Py<AudioSource>,
) -> PyResult<songbird::input::Input> {
    let songbird_source = source.call_method0(py, intern!(py, "_get_songbird_source"))?;
    let songbird_source = songbird_source.cast_bound::<SongbirdSource>(py)?;

    Ok(songbird_source.get().0.input()?)
}
