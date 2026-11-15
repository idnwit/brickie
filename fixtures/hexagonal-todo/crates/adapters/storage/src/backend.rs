//! Raw string storage: a file on native targets, `web_sys::Storage` on wasm32.

use todo_application::ports::outbound::RepoError;

/// Where the JSON snapshot lives.
pub enum Backend {
    /// Native file backend.
    #[cfg(not(target_arch = "wasm32"))]
    File(std::path::PathBuf),
    /// Browser localStorage under the given key.
    #[cfg(target_arch = "wasm32")]
    Browser(String),
}

impl Backend {
    /// Reads the stored text, or `None` when nothing was saved yet.
    pub(crate) fn read(&self) -> Result<Option<String>, RepoError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::File(path) => match std::fs::read_to_string(path) {
                Ok(text) => Ok(Some(text)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(RepoError(e.to_string())),
            },
            #[cfg(target_arch = "wasm32")]
            Self::Browser(key) => browser()?
                .get_item(key)
                .map_err(|_| RepoError("localStorage read failed".into())),
        }
    }

    pub(crate) fn write(&self, text: &str) -> Result<(), RepoError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::File(path) => std::fs::write(path, text).map_err(|e| RepoError(e.to_string())),
            #[cfg(target_arch = "wasm32")]
            Self::Browser(key) => browser()?
                .set_item(key, text)
                .map_err(|_| RepoError("localStorage write failed".into())),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn browser() -> Result<web_sys::Storage, RepoError> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .ok_or_else(|| RepoError("localStorage unavailable".into()))
}
