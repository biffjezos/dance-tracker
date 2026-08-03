use serde::Serialize;
use compositor::{
    error::{ OperationError },
    graph::{ NodeValidation }
};
    /*
What the UI is told about one registered operation: its own descriptor
fields plus the category its metadata() declares - carried as a plain
string (not the enum) since this is the JS boundary, and added ahead of
any UI code reading it yet so a future generic menu/list grouping (Phase 4)
has it available without another engine change.
*/
#[derive(Serialize)]
struct OperationView {
    id: &'static str,
    menu: &'static str,
    label: &'static str,
    action: Option<&'static str>,
    ui_action: Option<&'static str>,
    create_node: Option<&'static str>,
    category: &'static str,
    submenu: Option<&'static str>,
}

/*
What the UI is told about one editable parameter of a node. The options list
comes from the operation, so a selector can never offer a value the operation
does not accept.
*/
#[derive(Serialize)]
struct ParameterView {
    name: &'static str,
    kind: &'static str,
    options: &'static [&'static str],
    step: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    group: Option<&'static str>,
    value: String,
}

/*
What the UI is told about one input of a node: the wire name the operation
declares, and the node currently feeding it, if any.
*/
#[derive(Serialize)]
struct InputView {
    name: &'static str,
    source: Option<u32>,
    // Which OutputKind tags (see OutputKind::as_str()) may be wired into
    // this input - empty means unrestricted (every real node is a valid
    // candidate).
    accepts: Vec<&'static str>,
}

/*
What the UI is told about one declared output of a node: its index (what
`set_patch_mapping` addresses it by), its human-readable label (see
`Operation::output_names`), and its OutputKind tag (see OutputKind::as_str()).
*/
#[derive(Serialize)]
struct OutputView {
    index: u32,
    name: String,
    kind: String,
}

/*
What the UI is told about one PATCH property's current mapping - which
REFERENCE output drives it, and how (REPLACE/ADD/SUBTRACT). `patch_mapping`
returns this (or nothing at all) per property.
*/
#[derive(Serialize)]
struct PatchMappingView {
    output_index: u32,
    mode: &'static str,
}
/*
Whether a node is safe to evaluate, translated from the engine's internal
NodeValidation (which carries NodeId, not JS-safe on its own) into a tag the
UI can match on plus a human-readable detail string - e.g. so the NODES
list can badge a node with a dangling or cyclic wire instead of the user
only finding out when the whole graph refuses to render.
*/
#[derive(Serialize)]
struct NodeValidationView {
    state: &'static str,
    detail: Option<String>,
}

impl From<NodeValidation> for NodeValidationView {
    fn from(state: NodeValidation) -> Self {
        match state {
            NodeValidation::Valid => Self { state: "valid", detail: None },
            NodeValidation::MissingInput(input) => Self {
                state: "missing_input",
                detail: Some(input.name().to_string()),
            },
            NodeValidation::UnknownInput(id) => Self {
                state: "unknown_input",
                detail: Some(id.index().to_string()),
            },
            NodeValidation::InvalidDependency(id) => Self {
                state: "invalid_dependency",
                detail: Some(id.index().to_string()),
            },
            NodeValidation::Cycle => Self { state: "cycle", detail: None },
        }
    }
}