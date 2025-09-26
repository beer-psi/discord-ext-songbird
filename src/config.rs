use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use pyo3::prelude::*;
use songbird::driver::DisposalThread;

use crate::constants::DISPOSAL_THREAD;
use crate::error::SongbirdError;

#[pyclass(module = "discord.ext.songbird._native")]
#[derive(Clone, Debug)]
pub struct Config {
    pub inner: Arc<RwLock<songbird::Config>>,
}

impl Config {
    pub fn deep_clone(&self) -> Self {
        let config = self.inner.read().unwrap().clone();

        Self {
            inner: Arc::new(RwLock::new(config)),
        }
    }

    pub fn into_inner(self) -> songbird::Config {
        Arc::into_inner(self.inner).unwrap().into_inner().unwrap()
    }
}

#[pymethods]
impl Config {
    #[new]
    fn new(py: Python) -> Self {
        let disposer = DISPOSAL_THREAD
            .get_or_init(py, || DisposalThread::run())
            .clone();

        py.detach(move || {
            let config = songbird::Config::default().disposer(disposer);

            Self {
                inner: Arc::new(RwLock::new(config)),
            }
        })
    }

    #[getter]
    fn crypto_mode(&self) -> PyResult<CryptoMode> {
        Ok(self.inner.read().unwrap().crypto_mode.try_into()?)
    }

    #[setter]
    fn set_crypto_mode(&mut self, value: CryptoMode) {
        self.inner.write().unwrap().crypto_mode = value.into();
    }

    #[getter]
    fn gateway_timeout(&self) -> Option<Duration> {
        self.inner.read().unwrap().gateway_timeout
    }

    #[setter]
    fn set_gateway_timeout(&mut self, value: Option<Duration>) {
        self.inner.write().unwrap().gateway_timeout = value;
    }

    #[getter]
    fn mix_mode(&self) -> MixMode {
        self.inner.read().unwrap().mix_mode.into()
    }

    #[setter]
    fn set_mix_mode(&mut self, value: MixMode) {
        self.inner.write().unwrap().mix_mode = value.into();
    }

    #[getter]
    fn preallocated_tracks(&self) -> usize {
        self.inner.read().unwrap().preallocated_tracks
    }

    #[setter]
    fn set_preallocated_tracks(&mut self, value: usize) {
        self.inner.write().unwrap().preallocated_tracks = value;
    }

    #[getter]
    fn driver_retry_strategy(&self) -> PyResult<RetryStrategy> {
        Ok(self
            .inner
            .read()
            .unwrap()
            .driver_retry
            .strategy
            .try_into()?)
    }

    #[setter]
    fn set_driver_retry_strategy(&mut self, value: RetryStrategy) {
        self.inner.write().unwrap().driver_retry.strategy = value.into();
    }

    #[getter]
    fn driver_retry_limit(&self) -> Option<usize> {
        self.inner.read().unwrap().driver_retry.retry_limit
    }

    #[setter]
    fn set_driver_retry_limit(&mut self, value: Option<usize>) {
        self.inner.write().unwrap().driver_retry.retry_limit = value;
    }

    #[getter]
    fn use_softclip(&self) -> bool {
        self.inner.read().unwrap().use_softclip
    }

    #[setter]
    fn set_use_softclip(&mut self, value: bool) {
        self.inner.write().unwrap().use_softclip = value;
    }

    #[getter]
    fn driver_timeout(&self) -> Option<Duration> {
        self.inner.read().unwrap().driver_timeout
    }

    #[setter]
    fn set_driver_timeout(&mut self, value: Option<Duration>) {
        self.inner.write().unwrap().driver_timeout = value;
    }
}

#[pyclass(module = "discord.ext.songbird._native", frozen)]
#[derive(Clone, Copy, Debug)]
pub enum CryptoMode {
    Aes256Gcm,
    XChaCha20Poly1305,
}

impl From<CryptoMode> for songbird::driver::CryptoMode {
    fn from(value: CryptoMode) -> Self {
        match value {
            CryptoMode::Aes256Gcm => songbird::driver::CryptoMode::Aes256Gcm,
            CryptoMode::XChaCha20Poly1305 => songbird::driver::CryptoMode::XChaCha20Poly1305,
        }
    }
}

impl TryFrom<songbird::driver::CryptoMode> for CryptoMode {
    type Error = SongbirdError;

    fn try_from(value: songbird::driver::CryptoMode) -> Result<Self, Self::Error> {
        match value {
            songbird::driver::CryptoMode::Aes256Gcm => Ok(CryptoMode::Aes256Gcm),
            songbird::driver::CryptoMode::XChaCha20Poly1305 => Ok(CryptoMode::XChaCha20Poly1305),
            _ => Err(SongbirdError::UnknownCryptoMode(value)),
        }
    }
}

#[pyclass(module = "discord.ext.songbird._native", frozen)]
#[derive(Clone, Copy, Debug)]
pub enum MixMode {
    Mono,
    Stereo,
}

impl From<MixMode> for songbird::driver::MixMode {
    fn from(value: MixMode) -> Self {
        match value {
            MixMode::Mono => songbird::driver::MixMode::Mono,
            MixMode::Stereo => songbird::driver::MixMode::Stereo,
        }
    }
}

impl From<songbird::driver::MixMode> for MixMode {
    fn from(value: songbird::driver::MixMode) -> Self {
        match value {
            songbird::driver::MixMode::Mono => MixMode::Mono,
            songbird::driver::MixMode::Stereo => MixMode::Stereo,
        }
    }
}

#[pyclass(module = "discord.ext.songbird._native", frozen)]
#[derive(Clone, Copy, Debug)]
pub enum RetryStrategy {
    _Every(Duration),
    _Backoff {
        min: Duration,
        max: Duration,
        jitter: f32,
    },
}

#[pymethods]
impl RetryStrategy {
    #[staticmethod]
    fn every(duration: Duration) -> Self {
        Self::_Every(duration)
    }

    #[staticmethod]
    #[pyo3(signature = (min = Duration::from_secs_f32(0.25), max = Duration::from_secs_f32(10.0), jitter = 0.1))]
    fn backoff(min: Duration, max: Duration, jitter: f32) -> Self {
        Self::_Backoff {
            min: min,
            max: max,
            jitter,
        }
    }

    fn __repr__(&self) -> String {
        format!("RetryStrategy({:?})", self)
    }
}

impl From<RetryStrategy> for songbird::driver::retry::Strategy {
    fn from(value: RetryStrategy) -> Self {
        match value {
            RetryStrategy::_Every(duration) => songbird::driver::retry::Strategy::Every(duration),
            RetryStrategy::_Backoff { min, max, jitter } => {
                songbird::driver::retry::Strategy::Backoff(
                    songbird::driver::retry::ExponentialBackoff { min, max, jitter },
                )
            }
        }
    }
}

impl TryFrom<songbird::driver::retry::Strategy> for RetryStrategy {
    type Error = SongbirdError;

    fn try_from(value: songbird::driver::retry::Strategy) -> Result<Self, Self::Error> {
        match value {
            songbird::driver::retry::Strategy::Every(duration) => {
                Ok(RetryStrategy::_Every(duration))
            }
            songbird::driver::retry::Strategy::Backoff(exponential_backoff) => {
                Ok(RetryStrategy::_Backoff {
                    min: exponential_backoff.min,
                    max: exponential_backoff.max,
                    jitter: exponential_backoff.jitter,
                })
            }
            _ => Err(SongbirdError::UnknownRetryStrategy(value)),
        }
    }
}
