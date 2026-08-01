// graph/validate.rs

use std::collections::HashSet;

use crate::compositor::{
    error::OperationError,
    input::Input,
};

use super::{
    Graph,
    node::NodeId,
};


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeValidation {
    /// Node is valid and all its dependencies are valid
    Valid,
    /// Node is missing a required input connection
    MissingInput(Input),
    /// Node references a node that doesn't exist (removed)
    UnknownInput(NodeId),
    /// Node depends on another node that is invalid
    InvalidDependency(NodeId),
    /// Node participates in a cycle
    Cycle,
}

#[derive(Clone, Debug)]
pub enum ValidationState {
    Dirty,
    Valid,
    Invalid(OperationError),
}

/// Complete validation result containing per-node validation states
#[derive(Clone, Debug)]
pub struct ValidationResult {
    /// Overall graph validation state
    pub graph_state: ValidationState,
    /// Per-node validation states, indexed by node index
    pub node_states: Vec<NodeValidation>,
}

impl ValidationResult {
    pub fn new(num_nodes: usize) -> Self {
        Self {
            graph_state: ValidationState::Dirty,
            node_states: vec![NodeValidation::Valid; num_nodes],
        }
    }
}


pub fn validate_graph(
    graph: &mut Graph,
) -> Result<(), OperationError> {

    if !matches!(
        graph.validation,
        ValidationState::Dirty
    ) {
        return match &graph.validation {
            ValidationState::Valid =>
                Ok(()),

            ValidationState::Invalid(e) =>
                Err(e.clone()),

            ValidationState::Dirty =>
                unreachable!(),
        };
    }

    let result = run_validation(graph);

    // Store the validation result in the graph
    graph.node_validation = result.node_states.clone();

    graph.validation =
        match &result.graph_state {
            ValidationState::Valid =>
                ValidationState::Valid,

            ValidationState::Invalid(e) =>
                ValidationState::Invalid(e.clone()),

            ValidationState::Dirty =>
                ValidationState::Dirty,
        };

    /*
    Only structural damage makes the graph itself unusable - a cycle, or a
    reference to a node that no longer exists. An input the user simply has
    not wired yet is a normal editing state: it stays visible per node, and
    the operation decides what it produces without one.
    */
    for (index, state) in graph.node_validation.iter().enumerate() {
        if is_structural_failure(*state) {
            return Err(operation_error_from_node_validation(index, *state));
        }
    }

    Ok(())
}

/// Whether a node state means the graph cannot be evaluated at all.
fn is_structural_failure(state: NodeValidation) -> bool {
    matches!(
        state,
        NodeValidation::Cycle | NodeValidation::UnknownInput(_)
    )
}

/// Convert a NodeValidation state to an OperationError for backward compatibility
fn operation_error_from_node_validation(index: usize, state: NodeValidation) -> OperationError {
    match state {
        NodeValidation::MissingInput(_) => {
            OperationError::MissingInput("Node missing input".to_string())
        }
        NodeValidation::UnknownInput(_) => {
            OperationError::UnknownNode
        }
        NodeValidation::InvalidDependency(_) => {
            OperationError::UnknownNode
        }
        NodeValidation::Cycle => {
            OperationError::Cycle(vec![index])
        }
        NodeValidation::Valid => {
            OperationError::UnknownNode // Should not happen
        }
    }
}


fn run_validation(
    graph: &Graph,
) -> ValidationResult {

    let num_nodes = graph.nodes.len();
    let mut result = ValidationResult::new(num_nodes);

    // First pass: detect cycles and unknown nodes
    let mut state = vec![VisitState::Unvisited; num_nodes];
    let mut path = Vec::new();
    let mut cycle_nodes = HashSet::new();
    let mut unknown_input_nodes = Vec::new();

    // Track which nodes have unknown inputs
    for index in 0..num_nodes {
        if let Some(node) = &graph.nodes[index] {
            for (_, input_node_id) in &node.inputs {
                // Check if the input node exists and has the right generation
                if graph.resolve(*input_node_id).is_none() {
                    // This node references a non-existent node
                    unknown_input_nodes.push((index, *input_node_id));
                }
            }
        }
    }

    // Detect cycles using DFS
    for index in 0..num_nodes {
        if state[index] == VisitState::Unvisited {
            if let Some(_node) = &graph.nodes[index] {
                let id = NodeId {
                    index: index as u32,
                    generation: graph.generations[index],
                };

                if let Err(cycle) = visit_cycle_detection(
                    graph,
                    id,
                    &mut state,
                    &mut path,
                    &mut cycle_nodes,
                ) {
                    // Mark all nodes in the cycle
                    for &cycle_index in &cycle {
                        cycle_nodes.insert(cycle_index);
                    }
                }
            }
        }
    }

    // Second pass: determine validation state for each node
    for index in 0..num_nodes {
        if let Some(node) = &graph.nodes[index] {
            // Check if this node is in a cycle
            if cycle_nodes.contains(&index) {
                result.node_states[index] = NodeValidation::Cycle;
                continue;
            }

            // Check if this node has unknown inputs
            let has_unknown_input = unknown_input_nodes.iter()
                .any(|&(node_index, _)| node_index == index);
            
            if has_unknown_input {
                // Find the specific unknown input
                for (_, input_node_id) in &node.inputs {
                    if graph.resolve(*input_node_id).is_none() {
                        result.node_states[index] = NodeValidation::UnknownInput(*input_node_id);
                        break;
                    }
                }
                continue;
            }

            // Report the first input the operation declares but has not been
            // wired to - the operation itself decides whether it can run anyway.
            let metadata = node.operation.metadata();

            let unwired = metadata.inputs.iter().find(|key| {
                !node.inputs.iter().any(|(wired, _)| wired == *key)
            });

            if let Some(key) = unwired {
                result.node_states[index] = NodeValidation::MissingInput(*key);
            }
        } else {
            // This node slot is empty (removed node)
            // We don't need to set validation state for non-existent nodes
            continue;
        }
    }

    // Third pass: propagate invalidity through dependencies
    propagate_invalidity(graph, &mut result);

    // The graph as a whole is only invalid when it is structurally broken.
    let structural_failure = result.node_states.iter()
        .copied()
        .enumerate()
        .find(|&(_, state)| is_structural_failure(state));

    result.graph_state = match structural_failure {
        Some((index, state)) => ValidationState::Invalid(
            operation_error_from_node_validation(index, state)
        ),
        None => ValidationState::Valid,
    };

    result
}


fn visit_cycle_detection(
    graph: &Graph,
    id: NodeId,
    state: &mut [VisitState],
    path: &mut Vec<NodeId>,
    cycle_nodes: &mut HashSet<usize>,
) -> Result<(), Vec<usize>> {

    let node = graph.resolve(id)
        .ok_or(Vec::new())?;

    let index = id.index as usize;

    state[index] = VisitState::Visiting;
    path.push(id);

    for (_, input) in &node.inputs {
        let _input_index = input.index as usize;

        // Skip if the input node doesn't exist
        if graph.resolve(*input).is_none() {
            continue;
        }

        match state[input.index as usize] {
            VisitState::Visiting => {
                // Found a cycle
                let start = path.iter()
                    .position(|n| n == input)
                    .unwrap();

                let mut cycle = path[start..]
                    .iter()
                    .map(|n| n.index as usize)
                    .collect::<Vec<_>>();

                cycle.push(input.index as usize);

                return Err(cycle);
            }

            VisitState::Unvisited => {
                if let Err(cycle) = visit_cycle_detection(
                    graph,
                    *input,
                    state,
                    path,
                    cycle_nodes,
                ) {
                    // Mark all nodes in the cycle
                    for &cycle_index in &cycle {
                        cycle_nodes.insert(cycle_index);
                    }
                    return Err(cycle);
                }
            }

            VisitState::Visited => {}
        }
    }

    path.pop();
    state[index] = VisitState::Visited;

    Ok(())
}


/// Propagate invalidity through the dependency graph.
/// If a node is invalid, all nodes that depend on it (directly or indirectly) become invalid -
/// but only for *structural* invalidity (Cycle, UnknownInput, and InvalidDependency itself).
/// MissingInput never seeds this - an input the user simply hasn't wired yet (very often an
/// entirely optional one, like MASK, which every masking-capable operation declares whether or
/// not anyone ever wires it) is normal mid-edit state, not something that should paint every
/// node downstream of it red. describeNodeValidation on the UI side already treats a node's own
/// MissingInput as no badge at all; without this exclusion, that leniency didn't survive one hop
/// downstream - literally any two-node chain through an operation with an unwired optional MASK
/// input (which is most operations, in most graphs) got its dependent flagged InvalidDependency.
fn propagate_invalidity(
    graph: &Graph,
    result: &mut ValidationResult,
) {
    let num_nodes = graph.nodes.len();
    
    // Build dependency graph: for each node, track which nodes depend on it
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); num_nodes];
    
    for index in 0..num_nodes {
        if let Some(node) = &graph.nodes[index] {
            for (_, input_node_id) in &node.inputs {
                let input_index = input_node_id.index as usize;
                if input_index < num_nodes {
                    dependents[input_index].push(index);
                }
            }
        }
    }

    // Find all initially invalid nodes - structural breakage only, see this
    // function's own doc comment for why MissingInput is excluded.
    let mut invalid_queue: Vec<usize> = Vec::new();
    for index in 0..num_nodes {
        if matches!(
            result.node_states[index],
            NodeValidation::Cycle | NodeValidation::UnknownInput(_)
        ) {
            invalid_queue.push(index);
        }
    }

    // BFS to propagate invalidity
    let mut visited = vec![false; num_nodes];
    while let Some(invalid_index) = invalid_queue.pop() {
        if visited[invalid_index] {
            continue;
        }
        visited[invalid_index] = true;

        // For each node that depends on this invalid node
        for &dependent_index in &dependents[invalid_index] {
            // Only propagate if the dependent is currently valid
            if matches!(result.node_states[dependent_index], NodeValidation::Valid) {
                let invalid_node_id = NodeId {
                    index: invalid_index as u32,
                    generation: graph.generations[invalid_index],
                };
                result.node_states[dependent_index] = NodeValidation::InvalidDependency(invalid_node_id);
                invalid_queue.push(dependent_index);
            }
        }
    }
}


/// Get the validation state of a specific node
pub fn get_node_validation(
    graph: &Graph,
    id: NodeId,
) -> Option<NodeValidation> {
    let index = id.index as usize;
    
    // Check if the node exists and has the right generation
    if graph.generations.get(index) != Some(&id.generation) {
        return None;
    }
    
    if index >= graph.node_validation.len() {
        return None;
    }

    Some(graph.node_validation[index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use crate::compositor::{
        metadata::{OperationCategory, OperationMetadata, OutputKind},
        operations::Operation,
        operation_descriptor::OperationDescriptor,
        Context,
        Value,
    };

    /// A stub operation that declares whatever inputs the test needs -
    /// only its declared `inputs` list and wiring matter here, never its
    /// (never-called) execute() output.
    struct Stub {
        inputs: Vec<Input>,
    }

    impl Operation for Stub {
        fn descriptor(&self) -> OperationDescriptor {
            OperationDescriptor {
                id: "stub", menu: "TEST", label: "STUB",
                action: None, ui_action: None, create_node: None, submenu: None,
            }
        }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "Stub",
                category: OperationCategory::Color,
                inputs: self.inputs.clone(),
                outputs: vec![OutputKind::Image],
            }
        }
        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![])
        }
    }

    fn stub(inputs: Vec<Input>) -> Box<dyn Operation> {
        Box::new(Stub { inputs })
    }

    #[test]
    fn an_unwired_optional_input_does_not_cascade_to_a_dependent_node() {
        // Regression: a node with a declared-but-unwired input (MASK is
        // the real-world case - every masking-capable operation declares
        // it, and it's almost always left unwired) used to poison every
        // node downstream of it with a red InvalidDependency badge, even
        // though the node's own MissingInput state is normal, expected
        // mid-edit state, not an error.
        let mut graph = Graph::new(4, 4);

        let a = graph.add_node(stub(vec![])); // no inputs - Valid

        // b declares Source + Mask, but only Source gets wired - Mask
        // stays a normal unwired optional input.
        let b = graph.add_node(stub(vec![Input::Source, Input::Mask]));
        graph.connect(b, Input::Source, a).unwrap();

        // c depends on b - before the fix, this became InvalidDependency(b)
        // purely because b had an unwired (optional) Mask input.
        let c = graph.add_node(stub(vec![Input::Source]));
        graph.connect(c, Input::Source, b).unwrap();

        graph.validate().expect("MissingInput must not fail graph validation");

        assert_eq!(graph.node_validation(b), Some(NodeValidation::MissingInput(Input::Mask)));
        assert_eq!(graph.node_validation(c), Some(NodeValidation::Valid));
    }

    #[test]
    fn a_genuine_cycle_is_still_flagged_and_still_fails_validation() {
        // propagate_invalidity's own exclusion of MissingInput must not
        // touch real structural breakage - a cycle must still fail
        // validate() and mark both participants Cycle.
        //
        // Deliberately not extended with a third node depending on this
        // cycle (e.g. C -> B): run_validation's DFS state (`path`/`state`
        // in visit_cycle_detection) is never reset between separate
        // top-level traversal roots, so a node visited after a real cycle
        // elsewhere in the graph can itself be misflagged Cycle via a
        // stale leftover `path` - a separate, pre-existing bug, not
        // something this test (about MissingInput exclusion) should
        // depend on either way.
        let mut graph = Graph::new(4, 4);

        let a = graph.add_node(stub(vec![Input::Source]));
        let b = graph.add_node(stub(vec![Input::Source]));
        graph.connect(a, Input::Source, b).unwrap();
        graph.connect(b, Input::Source, a).unwrap();

        graph.validate().unwrap_err();

        assert_eq!(graph.node_validation(a), Some(NodeValidation::Cycle));
        assert_eq!(graph.node_validation(b), Some(NodeValidation::Cycle));
    }
}