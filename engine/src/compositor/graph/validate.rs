// graph/validate.rs

use crate::compositor::error::OperationError;

use super::{
    Graph,
    node::NodeId,
};


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}


#[derive(Clone)]
pub enum ValidationState {
    Dirty,
    Valid,
    Invalid(OperationError),
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


    graph.validation =
        match &result {
            Ok(_) =>
                ValidationState::Valid,

            Err(e) =>
                ValidationState::Invalid(e.clone()),
        };


    result
}



fn run_validation(
    graph: &Graph,
) -> Result<(), OperationError> {


    let mut state =
        vec![
            VisitState::Unvisited;
            graph.nodes.len()
        ];


    for index in 0..graph.nodes.len() {

        if state[index] ==
            VisitState::Unvisited
        {

            let id = NodeId {
                index: index as u32,
                generation:
                    graph.generations[index],
            };


            visit(
                graph,
                id,
                &mut state,
                &mut Vec::new(),
            )?;
        }
    }


    Ok(())
}



fn visit(
    graph: &Graph,
    id: NodeId,
    state: &mut [VisitState],
    path: &mut Vec<NodeId>,
)
-> Result<(), OperationError>
{

    let node =
        graph.resolve(id)
        .ok_or(OperationError::UnknownNode)?;


    let index = id.index as usize;


    state[index] =
        VisitState::Visiting;


    path.push(id);



    for (_, input) in &node.inputs {

        let input_index =
            input.index as usize;


        if graph.resolve(*input).is_none() {
            return Err(
                OperationError::UnknownNode
            );
        }


        match state[input_index] {

            VisitState::Visiting => {

                let start =
                    path.iter()
                    .position(|n| n == input)
                    .unwrap();


                let mut cycle =
                    path[start..]
                    .iter()
                    .map(|n|
                        n.index as usize)
                    .collect::<Vec<_>>();


                cycle.push(input_index);


                return Err(
                    OperationError::Cycle(cycle)
                );
            }


            VisitState::Unvisited => {

                visit(
                    graph,
                    *input,
                    state,
                    path,
                )?;
            }


            VisitState::Visited => {}
        }
    }


    path.pop();

    state[index] =
        VisitState::Visited;


    Ok(())
}