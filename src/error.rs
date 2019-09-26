use std::io;
use crate::ResponseStatus;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Communication(ResponseStatus),
    Protocol(ResponseStatus),
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

            ResponseStatus::WrongLength => Error::Program("Wrong command length".to_string()),
            ResponseStatus::IllegalCommand => Error::Program("Illegal command".to_string()),
            ResponseStatus::ParameterError => Error::Program("Parameter error".to_string()),

            other => Error::Program(format!("Invalid status response: {:?}", other))
        }
    }
}
