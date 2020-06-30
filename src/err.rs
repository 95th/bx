use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    Eof,
    Type { reason: &'static str },
    Length { expected: usize, actual: usize },
    Parse { reason: &'static str, pos: usize },
    Unexpected { pos: usize },
    Overflow { pos: usize },
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Eof => f.write_str("Unexpected end of file"),
            Error::Type { reason } => write!(f, "Type Mismatch: {}", reason),
            Error::Length { expected, actual } => write!(
                f,
                "Length Mismatch: Expected: {}, Actual: {}",
                expected, actual
            ),
            Error::Parse { reason, pos } => write!(f, "Parse Error at {}: {}", pos, reason),
            Error::Unexpected { pos } => write!(f, "Unexpected character at {}", pos),
            Error::Overflow { pos } => write!(f, "Numeric overflow occurred at {}", pos),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
