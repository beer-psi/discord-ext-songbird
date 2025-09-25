use pyo3::{
    create_exception,
    exceptions::{PyException, PyTimeoutError},
    PyErr,
};
use songbird::error::{ControlError, JoinError, PlayError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SongbirdError {
    #[error("Connection was not started")]
    ConnectionNotStarted,

    #[error("Unknown crypto mode: {0:?}")]
    UnknownCryptoMode(songbird::driver::CryptoMode),

    #[error("Unknown driver retry strategy: {0:?}")]
    UnknownRetryStrategy(songbird::driver::retry::Strategy),

    #[error("Unknown track event: {0:?}")]
    UnknownTrackEvent(songbird::events::TrackEvent),

    #[error("Gateway connection error: {0:?}")]
    JoinError(#[from] JoinError),

    #[error("Track control error: {0:?}")]
    ControlError(#[from] ControlError),

    #[error("Play error: {0:?}")]
    PlayError(#[from] PlayError),

    #[error("Source has been consumed and is no longer valid")]
    SourceConsumed,
}

create_exception!(
    discord.ext.songbird._native.exceptions,
    PySongbirdError,
    PyException
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyConnectionNotStarted,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyUnknownCryptoMode,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyUnknownRetryStrategy,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyUnknownTrackEvent,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyJoinError,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyControlError,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PyPlayError,
    PySongbirdError
);
create_exception!(
    discord.ext.songbird._native.exceptions,
    PySourceConsumed,
    PySongbirdError
);

impl From<SongbirdError> for PyErr {
    fn from(value: SongbirdError) -> Self {
        match value {
            SongbirdError::ConnectionNotStarted => PyConnectionNotStarted::new_err(()),
            SongbirdError::UnknownCryptoMode(_) => PyUnknownCryptoMode::new_err(value.to_string()),
            SongbirdError::UnknownRetryStrategy(_) => {
                PyUnknownRetryStrategy::new_err(value.to_string())
            }
            SongbirdError::UnknownTrackEvent(_) => PyUnknownTrackEvent::new_err(value.to_string()),
            SongbirdError::JoinError(inner) => match inner {
                JoinError::TimedOut => PyTimeoutError::new_err("Timed out connecting to Discord"),
                _ => PyJoinError::new_err(inner.to_string()),
            },
            SongbirdError::ControlError(_) => PyControlError::new_err(value.to_string()),
            SongbirdError::PlayError(_) => PyPlayError::new_err(value.to_string()),
            SongbirdError::SourceConsumed => PySourceConsumed::new_err(value.to_string()),
        }
    }
}

pub type SongbirdResult<T> = Result<T, SongbirdError>;
