use hyper::http::uri::InvalidUri;
use hyper::Error;
use log::SetLoggerError;
use std::fmt::{Display, Formatter};
use std::net::AddrParseError;
use tokio::sync::broadcast::error::{RecvError, SendError};
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::modules::core::context::CoreError;

#[derive(Debug)]
pub enum HapiError {
    SetLoggerError(SetLoggerError),
    InvalidUri(InvalidUri),
    HyperError(Error),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
    AddressParseError(AddrParseError),
    RouteAlreadyExists,
    RouteNotExists,
    MessageSendError(SendError<Command>),
    CoreError(CoreError),
    MessageReceiveError(RecvError),
}

impl Display for HapiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HapiError::SetLoggerError(set_logger_error) => write!(f, "{:?}", set_logger_error),
            HapiError::InvalidUri(invalid_uri) => write!(f, "{:?}", invalid_uri),
            HapiError::HyperError(hyper_error) => write!(f, "{:?}", hyper_error),
            HapiError::IoError(io_error) => write!(f, "{:?}", io_error),
            HapiError::SerdeError(serde_error) => write!(f, "{:?}", serde_error),
            HapiError::AddressParseError(address_parse_error) => {
                write!(f, "{:?}", address_parse_error)
            },
            HapiError::RouteAlreadyExists | HapiError::RouteNotExists => todo!(),
            HapiError::MessageSendError(tokio_send_msg_error) => {
                write!(f, "{:?}", tokio_send_msg_error)
            },
            HapiError::CoreError(core_error) => write!(f, "{:?}", core_error),
            HapiError::MessageReceiveError(recv_error) => write!(f, "{:?}", recv_error),
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

impl From<Error> for HapiError {
    fn from(hyper_error: Error) -> Self {
        HapiError::HyperError(hyper_error)
    }
}

impl From<std::io::Error> for HapiError {
    fn from(io_error: std::io::Error) -> Self {
        HapiError::IoError(io_error)
    }
}

impl From<serde_json::Error> for HapiError {
    fn from(serde_error: serde_json::Error) -> Self {
        HapiError::SerdeError(serde_error)
    }
}

impl From<AddrParseError> for HapiError {
    fn from(address_parse_error: AddrParseError) -> Self {
        HapiError::AddressParseError(address_parse_error)
    }
}

impl From<SendError<Command>> for HapiError {
    fn from(tokio_send_msg_error: SendError<Command>) -> Self {
        HapiError::MessageSendError(tokio_send_msg_error)
    }
}

impl From<CoreError> for HapiError {
    fn from(core_error: CoreError) -> Self {
        HapiError::CoreError(core_error)
    }
}

impl From<RecvError> for HapiError {
    fn from(recv_error: RecvError) -> Self {
        HapiError::MessageReceiveError(recv_error)
    }
}