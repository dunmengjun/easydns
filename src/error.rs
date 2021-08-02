use std::error::Error;
use std::{fmt, error};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;


#[derive(Debug, Clone)]
pub struct PanicError;

impl fmt::Display for PanicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "thread panic error")
    }
}

impl Error for PanicError {
    fn description(&self) -> &str {
        "thread panic error"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}