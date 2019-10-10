///! Error types
use std::io;
use failure::Fail;
use crate::ResponseStatus;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display="Reader I/O error")]
    Io(#[fail(cause)]io::Error),
    #[fail(display="Transient error communicating with tag: {:?}", _0)]
    Communication(ResponseStatus),
    #[fail(display="Error returned from tag: {:?}", _0)]
    Protocol(ResponseStatus),
    #[fail(display="Program error: {}", _0)]
    Program(String),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<String> for Error {
    fn from(e: String) -> Error {
        Error::Program(e)
    }
}

impl From<ResponseStatus> for Error {
    fn from(e: ResponseStatus) -> Error {
        match e {
            ResponseStatus::PoorCommunication => Error::Communication(e),
            ResponseStatus::NoTags => Error::Communication(e),

            ResponseStatus::AccessPasswordError => Error::Protocol(e),
            ResponseStatus::KillTagError => Error::Protocol(e),
            ResponseStatus::KillPasswordZero => Error::Protocol(e),
            ResponseStatus::CommandNotSupported => Error::Protocol(e),

            ResponseStatus::WrongLength => Error::Program("Wrong command length".to_string()),
            ResponseStatus::IllegalCommand => Error::Program("Illegal command".to_string()),
            ResponseStatus::ParameterError => Error::Program("Parameter error".to_string()),

            other => Error::Program(format!("Invalid status response: {:?}", other))
        }
    }
}
