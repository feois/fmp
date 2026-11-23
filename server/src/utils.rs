
use std::fmt::Display;
use thiserror::Error;

pub use tokio::io;

pub trait ResultUtil<T, E>: Sized {
    fn log(self) -> Option<T> where E: Display { self.log_with(|e| log::error!("{}", e)) }
    fn log_with(self, f: impl FnOnce(E)) -> Option<T>;
    fn log_discard(self) where E: Display { self.log(); }
    fn log_discard_with(self, f: impl FnOnce(E)) { self.log_with(f); }
    fn ignore(self) {}
}

impl<T, E> ResultUtil<T, E> for Result<T, E> {
    fn log_with(self, f: impl FnOnce(E)) -> Option<T> {
        match self.into() {
            Ok(t) => Some(t),
            Err(e) => { f(e); None }
        }
    }
}

#[derive(Error, Debug)]
#[error("IO Error: {0}")]
pub struct IOError(#[from] pub io::Error);
