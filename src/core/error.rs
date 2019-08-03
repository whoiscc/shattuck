//

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "type mismatch")]
    TypeMismatch,
    #[fail(display = "object is used by others")]
    BusyObject,
    #[fail(display = "not callable")]
    NotCallable,
    #[fail(display = "pop empty stack")]
    ExhaustedFrame,
    #[fail(display = "no parent frame")]
    NoParentFrame,
}
