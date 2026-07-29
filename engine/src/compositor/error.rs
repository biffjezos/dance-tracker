// src/compositor/error.rs

use std::fmt;

#[derive(Debug, Clone)]
pub enum OperationError {
    MissingInput(String),
    WrongValueType,
    DimensionMismatch,
    SourceNotFound(String),
    Cycle(Vec<usize>),
    UnknownParameter(String),
    UnknownNode,
    NotImplemented(String),
    InvalidInputType(String),
    InvalidParameterType(String),
    InvalidParameterValue(String, String),
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationError::MissingInput(msg) => write!(f, "Missing input: {}", msg),
            OperationError::WrongValueType => write!(f, "Wrong value type"),
            OperationError::DimensionMismatch => write!(f, "Dimension mismatch"),
            OperationError::SourceNotFound(msg) => write!(f, "Source not found: {}", msg),
            OperationError::Cycle(nodes) => write!(f, "Cycle detected in nodes: {:?}", nodes),
            OperationError::UnknownParameter(msg) => write!(f, "Unknown parameter: {}", msg),
            OperationError::UnknownNode => write!(f, "Unknown node"),
            OperationError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            OperationError::InvalidInputType(msg) => write!(f, "Invalid input type: {}", msg),
            OperationError::InvalidParameterType(msg) => write!(f, "Invalid parameter type: {}", msg),
            OperationError::InvalidParameterValue(name, value) => write!(f, "Invalid parameter value for {}: {}", name, value),
        }
    }
}
