use crate::deps::thiserror;



#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an io error occurred: {source}")]
    IO {
        #[from]
        source: std::io::Error,
        #[cfg(feature = "nightly")]
        backtrace: std::backtrace::Backtrace,
    },

    #[error("an error occurred casting between integer types: {source}")]
    Number{
        #[from] source: std::num::TryFromIntError,
        #[cfg(feature = "nightly")]
        backtrace: std::backtrace::Backtrace,
    },

    #[error("parsing {typename} from {value:?}, reason: {reason:}")]
    Parse {
        value:    String,
        typename: &'static str,
        reason:   String,
    },
    #[error("unknown error")]
    Unknown,
}
