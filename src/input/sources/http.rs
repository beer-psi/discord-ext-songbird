use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use songbird::input::HttpRequest as SongbirdHttpRequest;

use crate::{
    constants::HTTP_CLIENT,
    error::{SongbirdError, SongbirdResult},
    input::sources::base::{AudioSource, IntoSongbirdInput, SongbirdSource},
};

#[pyclass(extends=AudioSource)]
#[derive(Clone, Debug)]
pub struct HttpRequest {
    inner: Arc<RwLock<Option<SongbirdHttpRequest>>>,
}

#[pymethods]
impl HttpRequest {
    #[new]
    fn new(py: Python, url: String) -> (Self, AudioSource) {
        (
            Self {
                inner: Arc::new(RwLock::new(Some(SongbirdHttpRequest::new(
                    HTTP_CLIENT
                        .get_or_init(py, || reqwest::Client::new())
                        .clone(),
                    url,
                )))),
            },
            AudioSource::new(),
        )
    }

    #[staticmethod]
    fn with_headers(
        py: Python,
        url: String,
        headers: HashMap<String, Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let mut header_map = HeaderMap::new();

        for (key, value) in headers {
            let Ok(key) = key.parse::<HeaderName>() else {
                return Err(PyValueError::new_err(format!("invalid header name {key}")));
            };
            let values = if let Ok(value) = value.extract::<String>(py) {
                vec![value]
            } else if let Ok(values) = value.extract::<Vec<String>>(py) {
                values
            } else {
                return Err(PyTypeError::new_err(
                    "header values must be str or list[str]",
                ));
            };

            for value in values {
                let Ok(value) = value.parse::<HeaderValue>() else {
                    return Err(PyValueError::new_err(format!(
                        "invalid header value {value}"
                    )));
                };
                header_map.insert(key.clone(), value);
            }
        }

        let base = PyClassInitializer::from(AudioSource::new());
        let sub = base.add_subclass(Self {
            inner: Arc::new(RwLock::new(Some(SongbirdHttpRequest::new_with_headers(
                HTTP_CLIENT
                    .get_or_init(py, || reqwest::Client::new())
                    .clone(),
                url,
                header_map,
            )))),
        });

        Ok(Py::new(py, sub)?.into_any())
    }

    fn _get_songbird_source(&self) -> SongbirdResult<SongbirdSource> {
        Ok(SongbirdSource(Box::new(self.clone())))
    }
}

impl IntoSongbirdInput for HttpRequest {
    fn input(&self) -> SongbirdResult<songbird::input::Input> {
        let songbird_http_request = self.inner.write().unwrap().take();

        if let Some(songbird_http_request) = songbird_http_request {
            Ok(songbird_http_request.into())
        } else {
            Err(SongbirdError::SourceConsumed)
        }
    }
}
