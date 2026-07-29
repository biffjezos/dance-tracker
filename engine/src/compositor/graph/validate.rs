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

    // If any node is invalid, return an error
    if graph.node_validation.iter().any(|&state| !matches!(state, NodeValidation::Valid)) {
        // Find the first error to return
        for (index, state) in graph.node_validation.iter().enumerate() {
            if !matches!(state, NodeValidation::Valid) {
                return Err(operation_error_from_node_validation(index, *state));
            }
        }
    }

    Ok(())
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

            // Check if this node is missing required inputs
            // Use metadata to determine how many inputs are expected
            let metadata = node.operation.metadata();
            let expected_inputs = metadata.input_count;
            let actual_inputs = node.inputs.len();
            
            if expected_inputs > 0 && actual_inputs == 0 {
                // Node expects inputs but has none - mark as missing input
                // We'll use a generic input for now since we don't know which specific input is missing
                result.node_states[index] = NodeValidation::MissingInput(Input::Source);
            } else if actual_inputs < expected_inputs {
                // Node has some inputs but not enough - mark as missing input
                result.node_states[index] = NodeValidation::MissingInput(Input::Source);
            }
        } else {
            // This node slot is empty (removed node)
            // We don't need to set validation state for non-existent nodes
            continue;
        }
    }

    // Third pass: propagate invalidity through dependencies
    propagate_invalidity(graph, &mut result);

    // Determine overall graph state
    let has_invalid_nodes = result.node_states.iter()
        .any(|&state| !matches!(state, NodeValidation::Valid));
    
    if has_invalid_nodes {
        result.graph_state = ValidationState::Invalid(OperationError::UnknownNode);
    } else {
        result.graph_state = ValidationState::Valid;
    }

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


/// Propagate invalidity through the dependency graph
/// If a node is invalid, all nodes that depend on it (directly or indirectly) become invalid
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

    // Find all initially invalid nodes
    let mut invalid_queue: Vec<usize> = Vec::new();
    for index in 0..num_nodes {
        if !matches!(result.node_states[index], NodeValidation::Valid) {
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