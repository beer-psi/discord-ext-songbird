use std::io::{Read, Seek};

use async_trait::async_trait;
use pyo3::{exceptions::PyNotImplementedError, intern, prelude::*};
use songbird::input::{AudioStream, AudioStreamError, Compose, Input};
use symphonia::core::{io::MediaSource, probe::Hint};

use crate::{
    error::SongbirdResult,
    input::sources::base::{AudioSource, IntoSongbirdInput, SongbirdSource},
};

/// A raw audio source, similar to :class:`discord.player.AudioSource`.
/// This can be subclassed to provide custom audio implementations.
///
/// At a minimum, any subclasses must implement :meth:`read`.
#[pyclass(from_py_object, extends=AudioSource, subclass)]
#[derive(Clone, Debug)]
pub struct RawAudioSource {}

#[pymethods]
impl RawAudioSource {
    #[new]
    fn new() -> (Self, AudioSource) {
        (Self {}, AudioSource::new())
    }

    /// The MIME type of this audio source. Used to give a hint to Symphonia
    /// on what the audio type is. No hint is provided if not implemented.
    fn mime_type(&self) -> Option<String> {
        None
    }

    /// Returns the length in bytes of the source, if available.
    fn length(&self) -> Option<u64> {
        None
    }

    /// Subclasses must implement this.
    ///
    /// Reads some audio from the source. This can be in any format
    /// that Symphonia supports under the hood. Opus is guaranteed
    /// to be supported.
    fn read(&self) -> PyResult<Vec<u8>> {
        return Err(PyNotImplementedError::new_err(()));
    }

    /// Seeks to the given byte offset, interpreted relative to the position
    /// indicated by whence. Returns the new absolute position from the start.
    ///
    /// Similar to the standard :meth:`IOBase.seek` method.
    #[allow(unused_variables)]
    #[pyo3(signature = (offset, whence = 0))]
    fn seek(&self, offset: usize, whence: i32) -> PyResult<u64> {
        return Err(PyNotImplementedError::new_err(()));
    }

    /// Whether the audio source supports seeking. :meth:`seek` will not be called
    /// if this returns false, or raises an exception.
    fn seekable(&self) -> bool {
        false
    }

    /// Called after the process is done playing audio. Any exceptions
    /// that happens here will be ignored.
    fn close(&self) {}

    fn _get_songbird_source(slf: Py<RawAudioSource>, py: Python) -> SongbirdResult<SongbirdSource> {
        Ok(SongbirdSource(Box::new(RawAudioSourceInner(
            slf.clone_ref(py),
        ))))
    }
}

#[derive(Debug)]
struct RawAudioSourceInner(Py<RawAudioSource>);

impl Clone for RawAudioSourceInner {
    fn clone(&self) -> Self {
        Self(Python::attach(|py| self.0.clone_ref(py)))
    }
}

impl IntoSongbirdInput for RawAudioSourceInner {
    fn input(&self) -> SongbirdResult<Input> {
        Ok(Input::Lazy(Box::new(self.clone())))
    }
}

#[async_trait]
impl Compose for RawAudioSourceInner {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        let mime_type = Python::attach(|py| {
            let mime_type = self.0.call_method0(py, intern!(py, "mime_type"))?;
            let mime_type = mime_type.extract::<Option<String>>(py)?;

            PyResult::<Option<String>>::Ok(mime_type)
        })
        .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        Ok(AudioStream {
            input: Box::new(RawAudioSourceReader::new(Python::attach(|py| {
                self.0.clone_ref(py)
            }))),
            hint: mime_type.map(|m| {
                let mut hint = Hint::new();

                hint.mime_type(&m);
                hint
            }),
        })
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        Err(AudioStreamError::Unsupported)
    }

    fn should_create_async(&self) -> bool {
        false
    }
}

/// Struct implementing [`std::io::Read`] and [`std::io::Seek`] (not really)
/// for reading from a Python object with `read` and `close` methods.
// This is a different type since RawAudioSourceInner can be cloned freely;
// if the reader is merged into there the cleanup function will be called
// every clone drop, which we don't really want.
#[derive(Debug)]
struct RawAudioSourceReader {
    inner: Py<RawAudioSource>,
    length: Option<u64>,
    seekable: bool,
}

impl RawAudioSourceReader {
    pub fn new(inner: Py<RawAudioSource>) -> Self {
        let length = Python::attach(|py| {
            let length = inner.call_method0(py, intern!(py, "length"))?;
            let length = length.extract::<Option<u64>>(py)?;

            PyResult::<Option<u64>>::Ok(length)
        })
        .unwrap_or(None);

        let seekable = Python::attach(|py| {
            let seekable = inner.call_method0(py, intern!(py, "seekable"))?;
            let seekable = seekable.extract::<bool>(py)?;

            PyResult::<bool>::Ok(seekable)
        })
        .unwrap_or(false);

        Self {
            inner,
            length,
            seekable,
        }
    }
}

impl Read for RawAudioSourceReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data = Python::attach(|py| {
            let data = self.inner.call_method0(py, intern!(py, "read"))?;
            let data = data.extract::<Vec<u8>>(py)?;

            PyResult::<Vec<u8>>::Ok(data)
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        buf.copy_from_slice(&data);
        Ok(data.len())
    }
}

impl Seek for RawAudioSourceReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        if !self.seekable {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotSeekable,
                "not seekable",
            ));
        }

        let new_pos = Python::attach(|py| {
            let new_pos = match pos {
                std::io::SeekFrom::Start(offset) => {
                    self.inner
                        .call_method1(py, intern!(py, "seek"), (offset, 0))?
                }
                std::io::SeekFrom::Current(offset) => {
                    self.inner
                        .call_method1(py, intern!(py, "seek"), (offset, 1))?
                }
                std::io::SeekFrom::End(offset) => {
                    self.inner
                        .call_method1(py, intern!(py, "seek"), (offset, 2))?
                }
            };
            let new_pos = new_pos.extract::<u64>(py)?;

            PyResult::<u64>::Ok(new_pos)
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(new_pos)
    }
}

impl MediaSource for RawAudioSourceReader {
    fn is_seekable(&self) -> bool {
        self.seekable
    }

    fn byte_len(&self) -> Option<u64> {
        self.length
    }
}

impl Drop for RawAudioSourceReader {
    fn drop(&mut self) {
        // we don't really care if cleanup failed
        let _ = Python::attach(|py| self.inner.call_method0(py, intern!(py, "close")));
    }
}
