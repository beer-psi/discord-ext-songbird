use pyo3::sync::PyOnceLock;
use songbird::driver::DisposalThread;

pub static DISPOSAL_THREAD: PyOnceLock<DisposalThread> = PyOnceLock::new();
pub static HTTP_CLIENT: PyOnceLock<reqwest::Client> = PyOnceLock::new();
