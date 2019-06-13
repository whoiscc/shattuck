//

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "type mismatch")]
    TypeMismatch,
    #[fail(display = "synchronize rules violated")]
    ViolateSync,
    #[fail(display = "invalid memory address")]
    InvalidAddress,
    #[fail(display = "out of memory")]
    OutOfMemory,
    #[fail(display = "not callable")]
    NotCallable,
    #[fail(display = "not sharable")]
    NotSharable,
}

pub type Result<T> = std::result::Result<T, Error>;
