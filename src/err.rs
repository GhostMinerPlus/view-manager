use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Other(String),
    NotFound,
    RuntimeError,
}

pub type Result<T> = std::result::Result<T, moon_err::Error<ErrorKind>>;

pub fn map_edge_lib_err(
    stack: String,
) -> impl FnOnce(moon_err::Error<edge_lib::err::ErrorKind>) -> moon_err::Error<ErrorKind> {
    move |e| {
        log::error!("{e:?}\n{stack}");

        moon_err::Error::new(ErrorKind::RuntimeError, format!("{}", e.first().1), stack)
    }
}
