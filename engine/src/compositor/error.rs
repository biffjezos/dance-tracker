// src/compositor/error.rs

#[derive(Debug, Clone)]
pub enum OperationError {
    MissingInput,
    WrongValueType,
    DimensionMismatch,
    SourceNotFound(String),
    Cycle(Vec<usize>),
    UnknownParameter(String),
    UnknownNode,
}