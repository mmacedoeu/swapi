#[derive(Debug, ErrorChain)]
pub enum ErrorKind {
    Msg(String),

    #[error_chain(foreign)]
    Fmt(::std::fmt::Error),

    #[cfg(unix)]
    #[error_chain(foreign)]
    Io(::std::io::Error),

    #[error_chain(foreign)]
    Cli(::clap::Error),

    #[error_chain(foreign)]
    Pikkr(::pikkr::Error),

    #[error_chain(foreign)]
    Mentat(::mentat::errors::Error),

    #[error_chain(foreign)]
    Str(::std::str::Utf8Error),

    #[error_chain(foreign)]
    String(::std::string::FromUtf8Error),

    #[error_chain(foreign)]
    Parse(::std::num::ParseIntError),

    #[error_chain(foreign)]
    Failure(::failure::Compat<::failure::Error>),

    #[error_chain(custom)]
    Poisoned(String),
}