/*
A Node's inputs are positional (Vec<NodeId>), not named fields - what
position 0 vs 1 means is entirely up to the concrete Operation reading
them (e.g. Compose treats inputs[0] as the foreground, inputs[1] as
the background).
*/

use crate::compositor::{Operation, OperationError};

pub type NodeId = usize;

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<NodeId>,
}

pub struct Graph {
    pub nodes: Vec<Node>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

impl Graph {
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /*
    Every executor walks Node::inputs with plain recursion and no
    visited-set, so a cycle wouldn't error - it would recurse forever
    and blow the stack, which aborts the whole wasm instance (a trap,
    not a catchable Err) rather than failing one tick gracefully. Call
    this before handing the graph to an executor whenever its shape
    might have changed (app.rs does this in capture_background,
    render_tick and preview_tick).
    */
    pub fn validate(&self) -> Result<(), OperationError> {
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

        for &input_id in &self.nodes[id].inputs {
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

    struct NoOp;

    impl Operation for NoOp {
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

        fn execute(&self, _ctx: &Context, _inputs: &[Value]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![])
        }
    }

    fn node(inputs: Vec<NodeId>) -> Node {
        Node { operation: Box::new(NoOp), inputs }
    }

    #[test]
    fn empty_graph_is_valid() {
        let graph = Graph { nodes: vec![] };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn a_chain_with_no_cycle_is_valid() {
        let mut graph = Graph { nodes: vec![] };
        let a = graph.add_node(node(vec![]));
        let b = graph.add_node(node(vec![a]));
        graph.add_node(node(vec![b]));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn a_diamond_shared_input_is_not_a_cycle() {
        // c and d both depend on shared input a - fan-in, not a cycle.
        let mut graph = Graph { nodes: vec![] };
        let a = graph.add_node(node(vec![]));
        let c = graph.add_node(node(vec![a]));
        let d = graph.add_node(node(vec![a]));
        graph.add_node(node(vec![c, d]));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn a_node_pointing_at_itself_is_a_cycle() {
        let mut graph = Graph { nodes: vec![node(vec![])] };
        graph.nodes[0].inputs.push(0);

        let result = graph.validate();

        assert!(matches!(result, Err(OperationError::Cycle(path)) if path == vec![0, 0]));
    }

    #[test]
    fn a_longer_loop_is_a_cycle() {
        // a -> b -> c -> a, wired after the fact since add_node can only
        // reference already-existing ids - mirrors how a future
        // in-place rewire (rather than today's append-only add_node)
        // could actually produce this shape.
        let mut graph = Graph { nodes: vec![] };
        let a = graph.add_node(node(vec![]));
        let b = graph.add_node(node(vec![a]));
        let c = graph.add_node(node(vec![b]));
        graph.nodes[a].inputs.push(c);

        let result = graph.validate();

        assert!(matches!(result, Err(OperationError::Cycle(_))));
    }
}
