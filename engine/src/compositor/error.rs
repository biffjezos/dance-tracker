#[derive(Debug, Clone)]
pub enum OperationError {
    MissingInput,
    WrongValueType,
    DimensionMismatch,
    SourceNotFound(String),
    /*
    The offending cycle, node ids in traversal order (last id repeats
    the first, closing the loop) - not crate::graph::NodeId, to avoid
    graph.rs and compositor.rs importing each other; the two are the
    same underlying usize.
    */
    Cycle(Vec<usize>),
    // set_parameter with a name the operation doesn't have - distinct
    // from WrongValueType, which is a real parameter given a value of
    // the wrong kind.
    UnknownParameter(String),
    // A NodeId that doesn't resolve - out of range, or a stale
    // generation (its node has since been removed). Distinct from
    // MissingInput, which is a wire that was never connected at all.
    UnknownNode,
}