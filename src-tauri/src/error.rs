use serde::Serialize;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")] Io(#[from] std::io::Error),
    #[error("{0}")] Message(String),
}
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn message(error: impl std::fmt::Display) -> Error { Error::Message(error.to_string()) }
