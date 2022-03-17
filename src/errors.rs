use std::fmt::{Display, Formatter};
use hyper::Error;
use hyper::http::uri::InvalidUri;
use log::SetLoggerError;

#[derive(Debug)]
pub enum HapiError {
    SetLoggerError(SetLoggerError),
    InvalidUri(InvalidUri),
    HyperError(hyper::Error),
}

impl Display for HapiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HapiError::SetLoggerError(set_logger_error) => write!(f, "{:?}", set_logger_error),
            HapiError::InvalidUri(invalid_uri) => write!(f, "{:?}", invalid_uri),
            HapiError::HyperError(hyper_error) => write!(f, "{:?}", hyper_error),
        }
    }
}

impl std::error::Error for HapiError {}

impl From<SetLoggerError> for HapiError {
    fn from(set_logger_error: SetLoggerError) -> Self {
        HapiError::SetLoggerError(set_logger_error)
    }
}

impl From<InvalidUri> for HapiError {
    fn from(invalid_uri: InvalidUri) -> Self {
        HapiError::InvalidUri(invalid_uri)
    }
}

impl From<hyper::Error> for HapiError {
    fn from(hyper_error: Error) -> Self {
        HapiError::HyperError(hyper_error)
    }
}