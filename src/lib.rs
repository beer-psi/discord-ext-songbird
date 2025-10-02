mod bitrate;
mod client;
mod config;
mod connection;
mod constants;
mod error;
mod events;
mod input;
mod tracks;

use pyo3::prelude::*;

use crate::error::{
    PyConnectionInvalid, PyControlError, PyJoinError, PyPlayError, PySongbirdError,
    PySourceConsumed, PyUnknownCryptoMode, PyUnknownRetryStrategy, PyUnknownTrackEvent,
};

/// A Python module implemented in Rust.
#[pymodule(gil_used = false, name = "_native")]
fn discord_ext_songbird(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<client::SongbirdClient>()?;

    m.add_class::<bitrate::Bitrate>()?;

    m.add_class::<config::Config>()?;
    m.add_class::<config::CryptoMode>()?;
    m.add_class::<config::MixMode>()?;
    m.add_class::<config::RetryStrategy>()?;

    m.add_class::<events::track::TrackEvent>()?;

    m.add_class::<input::sources::base::AudioSource>()?;
    m.add_class::<input::sources::file::File>()?;
    m.add_class::<input::sources::http::HttpRequest>()?;
    m.add_class::<input::sources::raw::RawAudioSource>()?;

    m.add_class::<tracks::Track>()?;
    m.add_class::<tracks::handle::TrackHandle>()?;
    m.add_class::<tracks::looping::LoopState>()?;
    m.add_class::<tracks::mode::PlayMode>()?;

    m.add("SongbirdError", m.py().get_type::<PySongbirdError>())?;
    m.add(
        "ConnectionInvalid",
        m.py().get_type::<PyConnectionInvalid>(),
    )?;
    m.add(
        "UnknownCryptoMode",
        m.py().get_type::<PyUnknownCryptoMode>(),
    )?;
    m.add(
        "UnknownRetryStrategy",
        m.py().get_type::<PyUnknownRetryStrategy>(),
    )?;
    m.add(
        "UnknownTrackEvent",
        m.py().get_type::<PyUnknownTrackEvent>(),
    )?;
    m.add("JoinError", m.py().get_type::<PyJoinError>())?;
    m.add("ControlError", m.py().get_type::<PyControlError>())?;
    m.add("PlayError", m.py().get_type::<PyPlayError>())?;
    m.add("SourceConsumed", m.py().get_type::<PySourceConsumed>())?;

    Ok(())
}
