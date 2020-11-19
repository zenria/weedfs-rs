use crate::types::responses::ResponseError;
use std::error::Error;
use std::fmt::Formatter;
use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct WeedFSError {
    repr: WeedFSErrorRepr,
}

impl Error for WeedFSError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.repr {
            WeedFSErrorRepr::Simple(_) => None,
            WeedFSErrorRepr::Other(e) => e.source(),
        }
    }
}
impl Display for WeedFSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.repr {
            WeedFSErrorRepr::Simple(kind) => write!(f, "{:?}", kind),
            WeedFSErrorRepr::Other(error) => Display::fmt(error, f),
        }
    }
}

impl WeedFSError {
    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            WeedFSErrorRepr::Simple(kind) => kind,
            _ => ErrorKind::Other,
        }
    }
}
#[derive(Debug)]
enum WeedFSErrorRepr {
    Simple(ErrorKind),
    Other(anyhow::Error),
}

#[derive(Debug, Copy, Clone)]
pub enum ErrorKind {
    NotFound,
    Other,
}

impl From<anyhow::Error> for WeedFSError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            repr: WeedFSErrorRepr::Other(error),
        }
    }
}

impl From<ErrorKind> for WeedFSError {
    fn from(kind: ErrorKind) -> Self {
        Self {
            repr: WeedFSErrorRepr::Simple(kind),
        }
    }
}

impl From<ResponseError> for WeedFSError {
    fn from(e: ResponseError) -> Self {
        Self::from(anyhow::Error::from(e))
    }
}
