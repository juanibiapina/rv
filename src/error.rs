use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("git: {0}")]
    Git(String),
}
