/*
A Node's inputs are named (Vec<(Input, NodeId)>), not positional - what
each wire means is a label on the edge itself (e.g. Compose reads
Input::Foreground and Input::Background) rather than a convention the
caller and the operation had to independently agree on for position 0
vs 1.
*/

use std::any::Any;

use crate::compositor::{
    context::Context,
    error::OperationError,
    graph::Graph,
    input::Input,
    metadata::Meta,
    node::{ NodeId, Node },
    operations::{ Operation }, 
    metadata::OperationMetadata,
    value::Value
};

use crate::compositor::{

};
/*
A plain usize index would go stale silently once node removal exists -
reusing (or even just outliving) an index that now names a different
node. generation guards against that: add_node always mints
generation 0 for a brand new slot; remove_node (unused by anything
today - deletion isn't implemented, this only proves the mechanism
works) bumps the slot's generation without reusing it, so any NodeId
minted before the removal stops resolving. from_index() is for
boundary callers (app.rs) that only ever hand back an index they
previously received - JS never deletes a node, so it always effectively
means generation 0, and resolve() below still rejects it correctly if
the graph's own generation for that slot has since moved on.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    index: u32,
    generation: u32,
}

impl NodeId {
    pub fn from_index(index: u32) -> Self {
        NodeId { index, generation: 0 }
    }

    pub fn index(&self) -> u32 {
        self.index
    }
}

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
Everything a future save/load would need to reconstruct one node -
see Graph::describe_node. Not a serialization format itself (no
serde, no schema) - just proof the pieces (stable id, typed metadata,
parameters as Value data, explicit input wiring) already compose into
one self-describing snapshot.
*/
pub struct NodeDescription {
    pub id: NodeId,
    pub metadata: OperationMetadata,
    pub parameters: Vec<(&'static str, Value)>,
    pub inputs: Vec<(Input, NodeId)>,
}

/*
Owns the render resolution so it's one piece of state shared by every
node's execution (via Context - see App::context), instead of each
source/generator operation baking in its own fixed width/height at
construction time and needing the whole graph rebuilt to change it.
*/
pub struct Graph {
    pub nodes: Vec<Node>,
    // Parallel to nodes - generations[i] is the current generation for
    // slot i, bumped by remove_node, checked by resolve.
    generations: Vec<u32>,
    /*
    Which node is "the" render output, if the graph has designated one
    - informational, not enforced: render_tick/preview_tick still take
    an explicit node id and can target any node (arbitrary-node preview
    stays possible), this is for save/load and a future editor to know
    what "the" output is without a caller having to track it
    separately.
    */
    output: Option<NodeId>,
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
        Graph {
            nodes: vec![],
            generations: vec![],
            output: None,
            width,
            height,
            validation: ValidationState::Dirty,
        }
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn set_output(&mut self, node_id: NodeId) {
        self.output = Some(node_id);
    }

    pub fn output(&self) -> Option<NodeId> {
        self.output
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        let index = self.nodes.len() as u32;
        self.nodes.push(node);
        self.generations.push(0);
        self.validation = ValidationState::Dirty;
        NodeId { index, generation: 0 }
    }

    /*
    Not called from anywhere yet - app.rs/JS never removes a node today
    (rebuildGraph() always just re-adds, leaving orphans, a known
    accepted tradeoff). Exists so the generation mechanism is real and
    tested rather than an unused type change: the slot stays in nodes
    (a tombstone, not freed for reuse - an explicit free-list is its
    own follow-up if slot reuse is ever wanted), but its generation
    moves past whatever any already-minted NodeId for it remembers.
    */
    pub fn remove_node(&mut self, node_id: NodeId) -> bool {
        let idx = node_id.index as usize;

        if idx >= self.generations.len() || self.generations[idx] != node_id.generation {
            return false;
        }

        self.generations[idx] += 1;
        self.validation = ValidationState::Dirty;
        true
    }

    pub fn resolve(&self, node_id: NodeId) -> Option<&Node> {
        let idx = node_id.index as usize;

        if self.generations.get(idx) == Some(&node_id.generation) {
            self.nodes.get(idx)
        } else {
            None
        }
    }

    fn resolve_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        let idx = node_id.index as usize;

        if self.generations.get(idx) == Some(&node_id.generation) {
            self.nodes.get_mut(idx)
        } else {
            None
        }
    }

    /*
    Generalizes the as_any_mut/downcast_mut pattern already used for
    live-editing a node's own concrete Operation (THRESHOLD +/-, text
    content, ...) so a call site names the target type once instead of
    repeating as_any_mut().downcast_mut::<T>() inline. None covers "no
    node at this id" (unknown or stale), "wrong type" - callers that
    already know node_id names a T only care that the edit happened,
    not which reason a miss would be.
    */
    pub fn operation_mut<T: Any>(&mut self, node_id: NodeId) -> Option<&mut T> {
        self.resolve_mut(node_id)?.operation.as_any_mut().downcast_mut::<T>()
    }

    /*
    Prepares (without implementing) graph save/load: bundles everything
    a serializer would need for one node - a stable id, what kind of
    operation it is (metadata), its current settings as data (walking
    parameters() and reading each one back through get_parameter(),
    not just the descriptor list), and its explicit connections
    (inputs, unchanged). Nothing here reaches into a concrete Operation
    type - describe_node works for any Operation purely through the
    trait, which is the actual point: a future save/load or node
    editor never needs a per-kind branch to know what a node is or
    holds.
    */
    pub fn describe_node(&self, node_id: NodeId) -> Option<NodeDescription> {
        let node = self.resolve(node_id)?;

        let parameters = node
            .operation
            .parameters()
            .into_iter()
            .filter_map(|p| node.operation.get_parameter(p.name).map(|v| (p.name, v)))
            .collect();

        Some(NodeDescription {
            id: node_id,
            metadata: node.operation.metadata(),
            parameters,
            inputs: node.inputs.clone(),
        })
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
                self.visit(NodeId { index: start as u32, generation: self.generations[start] }, &mut state, &mut path)?;
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
        let idx = id.index as usize;

        state[idx] = VisitState::Visiting;
        path.push(id);

        let node = self.resolve(id).ok_or(OperationError::UnknownNode)?;

        for &(_, input_id) in &node.inputs {
            /*
            state below is positional (indexed by slot, not
            generation-checked), so a stale input_id pointing at an
            already-visited slot would otherwise read as "fine, already
            visited" without this - resolve() is the actual generation
            check.
            */
            if self.resolve(input_id).is_none() {
                return Err(OperationError::UnknownNode);
            }

            let input_idx = input_id.index as usize;

            match state[input_idx] {
                VisitState::Visiting => {
                    let start = path.iter().position(|&n| n == input_id).unwrap();
                    let mut cycle: Vec<usize> = path[start..].iter().map(|n| n.index as usize).collect();
                    cycle.push(input_idx);
                    return Err(OperationError::Cycle(cycle));
                }
                VisitState::Unvisited => self.visit(input_id, state, path)?,
                VisitState::Visited => {}
            }
        }

        path.pop();
        state[idx] = VisitState::Visited;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::{Context, OperationCategory, OperationMetadata, Value};

    // calls exists purely so operation_mut's tests have something
    // observable to mutate through the returned reference - cycle
    // detection itself never reads it.
    struct NoOp {
        calls: u32,
    }

    impl Operation for NoOp {
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "NoOp",
                category: OperationCategory::Reference,
                input_count: 0,
                outputs: vec![],
            }
        }

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

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "OtherOp",
                category: OperationCategory::Reference,
                input_count: 0,
                outputs: vec![],
            }
        }

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
        let a = graph.add_node(node(vec![]));
        graph.nodes[0].inputs.push((Input::Source, a));

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
        graph.nodes[a.index as usize].inputs.push((Input::Source, c));

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
        graph.nodes[a.index as usize].inputs.push((Input::Source, a));

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
    fn output_defaults_to_none_and_round_trips_through_set_output() {
        let mut graph = Graph::new(1, 1);
        assert_eq!(graph.output(), None);

        let id = graph.add_node(node(vec![]));
        graph.set_output(id);

        assert_eq!(graph.output(), Some(id));
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

        assert!(graph.operation_mut::<NoOp>(NodeId::from_index(0)).is_none());
    }

    #[test]
    fn removed_node_is_a_stale_reference_even_though_its_slot_still_exists() {
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));

        assert!(graph.operation_mut::<NoOp>(a).is_some(), "should resolve before removal");

        assert!(graph.remove_node(a), "should succeed removing a live node");

        assert!(
            graph.operation_mut::<NoOp>(a).is_none(),
            "the pre-removal NodeId should no longer resolve"
        );

        // The slot is a tombstone, not reused - the pre-removal NodeId's
        // index is still in range, only its generation is now stale.
        assert!(a.index() < graph.nodes.len() as u32);
    }

    #[test]
    fn remove_node_rejects_an_id_that_was_already_stale() {
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));

        assert!(graph.remove_node(a));
        assert!(!graph.remove_node(a), "removing the same (now stale) id again should fail");
    }

    #[test]
    fn a_node_referencing_a_removed_input_fails_validation_instead_of_panicking() {
        let mut graph = Graph::new(1, 1);
        let a = graph.add_node(node(vec![]));
        graph.add_node(node(vec![a]));

        graph.remove_node(a);

        assert!(matches!(graph.validate(), Err(OperationError::UnknownNode)));
    }

    #[test]
    fn describe_node_bundles_id_metadata_parameters_and_inputs() {
        use crate::operations::masks::{Chroma, Fill};

        let mut graph = Graph::new(1, 1);
        let source = graph.add_node(node(vec![]));

        let chroma_id = graph.add_node(Node {
            operation: Box::new(Chroma {
                key_colour: (0, 255, 0),
                threshold: 42,
                fill: Fill::Solid(255, 0, 255),
            }),
            inputs: vec![(Input::Source, source)],
        });

        let description = graph.describe_node(chroma_id).expect("should resolve");

        assert_eq!(description.id, chroma_id);
        assert_eq!(description.metadata.display_name, "Chroma Key");
        assert_eq!(description.inputs, vec![(Input::Source, source)]);

        assert_eq!(description.parameters.len(), 1);
        let (name, value) = &description.parameters[0];
        assert_eq!(*name, "threshold");
        assert!(matches!(value, Value::Number(v) if *v == 42.0));
    }

    #[test]
    fn describe_node_returns_none_for_an_unknown_node_id() {
        let graph = Graph::new(1, 1);

        assert!(graph.describe_node(NodeId::from_index(0)).is_none());
    }
}
