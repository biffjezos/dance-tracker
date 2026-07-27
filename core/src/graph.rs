/*
A Node's inputs are named (Vec<(Input, NodeId)>), not positional - what
each wire means is a label on the edge itself (e.g. Compose reads
Input::Foreground and Input::Background) rather than a convention the
caller and the operation had to independently agree on for position 0
vs 1.
*/

use std::any::Any;

use crate::compositor::{Input, Operation, OperationError};

pub type NodeId = usize;

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<(Input, NodeId)>,
}

impl Node {
    pub fn input(&self, key: Input) -> Option<NodeId> {
        self.inputs.iter().find(|(k, _)| *k == key).map(|(_, id)| *id)
    }
}

/*
Owns the render resolution so it's one piece of state shared by every
node's execution (via Context - see App::context), instead of each
source/generator operation baking in its own fixed width/height at
construction time and needing the whole graph rebuilt to change it.
*/
pub struct Graph {
    pub nodes: Vec<Node>,
    width: u32,
    height: u32,
    validation: ValidationState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

#[derive(Clone)]
enum ValidationState {
    Dirty,
    Valid,
    Invalid(OperationError),
}

impl Graph {
    pub fn new(width: u32, height: u32) -> Self {
        Graph { nodes: vec![], width, height, validation: ValidationState::Dirty }
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        self.validation = ValidationState::Dirty;
        id
    }

    /*
    Generalizes the as_any_mut/downcast_mut pattern already used for
    live-editing a node's own concrete Operation (THRESHOLD +/-, text
    content, ...) so a call site names the target type once instead of
    repeating as_any_mut().downcast_mut::<T>() inline. None covers both
    "no node at this id" and "wrong type" - callers that already know
    node_id names a T only care that the edit happened, not which of
    the two reasons a miss would be.
    */
    pub fn operation_mut<T: Any>(&mut self, node_id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(node_id)?.operation.as_any_mut().downcast_mut::<T>()
    }

    /*
    Every executor walks Node::inputs with plain recursion and no
    visited-set, so a cycle wouldn't error - it would recurse forever
    and blow the stack, which aborts the whole wasm instance (a trap,
    not a catchable Err) rather than failing one tick gracefully. Call
    this before handing the graph to an executor whenever its shape
    might have changed (app.rs does this in capture_background,
    render_tick and preview_tick) - cheap to call every time since the
    actual DFS below only re-runs when add_node marked the graph dirty
    since the last validate() call; a validated-then-untouched graph
    just returns the cached result.
    */
    pub fn validate(&mut self) -> Result<(), OperationError> {
        if matches!(self.validation, ValidationState::Dirty) {
            let result = self.run_validation();
            self.validation = match &result {
                Ok(()) => ValidationState::Valid,
                Err(e) => ValidationState::Invalid(e.clone()),
            };
            return result;
        }

        match &self.validation {
            ValidationState::Valid => Ok(()),
            ValidationState::Invalid(e) => Err(e.clone()),
            ValidationState::Dirty => unreachable!("just resolved above"),
        }
    }

    fn run_validation(&self) -> Result<(), OperationError> {
        let mut state = vec![VisitState::Unvisited; self.nodes.len()];

        for start in 0..self.nodes.len() {
            if state[start] == VisitState::Unvisited {
                let mut path = Vec::new();
                self.visit(start, &mut state, &mut path)?;
            }
        }

        Ok(())
    }

    fn visit(
        &self,
        id: NodeId,
        state: &mut [VisitState],
        path: &mut Vec<NodeId>,
    ) -> Result<(), OperationError> {
        state[id] = VisitState::Visiting;
        path.push(id);

        for &(_, input_id) in &self.nodes[id].inputs {
            match state[input_id] {
                VisitState::Visiting => {
                    let start = path.iter().position(|&n| n == input_id).unwrap();
                    let mut cycle = path[start..].to_vec();
                    cycle.push(input_id);
                    return Err(OperationError::Cycle(cycle));
                }
                VisitState::Unvisited => self.visit(input_id, state, path)?,
                VisitState::Visited => {}
            }
        }

        path.pop();
        state[id] = VisitState::Visited;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::{Context, Value};

    // calls exists purely so operation_mut's tests have something
    // observable to mutate through the returned reference - cycle
    // detection itself never reads it.
    struct NoOp {
        calls: u32,
    }

    impl Operation for NoOp {
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![])
        }
    }

    // A second concrete Operation type, distinct from NoOp, purely to
    // exercise operation_mut's wrong-type case.
    struct OtherOp;

    impl Operation for OtherOp {
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![])
        }
    }

    // Cycle detection only cares about graph connectivity, not which
    // role each edge plays, so every test edge is tagged the same -
    // real key choice is exercised by each operation's own tests.
    fn node(inputs: Vec<NodeId>) -> Node {
        Node {
            operation: Box::new(NoOp { calls: 0 }),
            inputs: inputs.into_iter().map(|id| (Input::Source, id)).collect(),
        }
    }

    #[test]
    fn empty_graph_is_valid() {
        let mut graph = Graph::new(1, 1);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn a_chain_with_no_cycle_is_valid() {
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));
        let b = graph.add_node(node(vec![a]));
        graph.add_node(node(vec![b]));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn a_diamond_shared_input_is_not_a_cycle() {
        // c and d both depend on shared input a - fan-in, not a cycle.
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));
        let c = graph.add_node(node(vec![a]));
        let d = graph.add_node(node(vec![a]));
        graph.add_node(node(vec![c, d]));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn a_node_pointing_at_itself_is_a_cycle() {
        let mut graph = Graph::new(1, 1);
        graph.add_node(node(vec![]));
        graph.nodes[0].inputs.push((Input::Source, 0));

        let result = graph.validate();

        assert!(matches!(result, Err(OperationError::Cycle(path)) if path == vec![0, 0]));
    }

    #[test]
    fn a_longer_loop_is_a_cycle() {
        // a -> b -> c -> a, wired after the fact since add_node can only
        // reference already-existing ids - mirrors how a future
        // in-place rewire (rather than today's append-only add_node)
        // could actually produce this shape.
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));
        let b = graph.add_node(node(vec![a]));
        let c = graph.add_node(node(vec![b]));
        graph.nodes[a].inputs.push((Input::Source, c));

        let result = graph.validate();

        assert!(matches!(result, Err(OperationError::Cycle(_))));
    }

    #[test]
    fn validate_result_is_cached_until_add_node_dirties_it_again() {
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));
        graph.add_node(node(vec![a]));

        assert!(graph.validate().is_ok());

        // Bypasses add_node, so this does NOT mark the graph dirty -
        // it introduces a real cycle that the cached Valid result
        // won't know about.
        graph.nodes[a].inputs.push((Input::Source, a));

        assert!(
            graph.validate().is_ok(),
            "cached result should be reused instead of re-running the DFS"
        );

        // add_node dirties the graph again, so this validate() call
        // actually re-runs the DFS and catches the cycle that's been
        // sitting there since the out-of-band mutation above.
        graph.add_node(node(vec![]));

        assert!(matches!(graph.validate(), Err(OperationError::Cycle(_))));
    }

    #[test]
    fn operation_mut_returns_the_concrete_type_and_allows_mutation() {
        let mut graph = Graph::new(1, 1);
        let id = graph.add_node(node(vec![]));

        graph.operation_mut::<NoOp>(id).expect("should find the NoOp").calls += 1;

        assert_eq!(graph.operation_mut::<NoOp>(id).unwrap().calls, 1);
    }

    #[test]
    fn operation_mut_returns_none_for_the_wrong_type() {
        let mut graph = Graph::new(1, 1);
        let id = graph.add_node(node(vec![])); // a NoOp, not an OtherOp

        assert!(graph.operation_mut::<OtherOp>(id).is_none());
    }

    #[test]
    fn operation_mut_returns_none_for_an_unknown_node_id() {
        let mut graph = Graph::new(1, 1);

        assert!(graph.operation_mut::<NoOp>(0).is_none());
    }
}
