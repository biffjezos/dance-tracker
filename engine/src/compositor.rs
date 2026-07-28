// biffjezos: THIS IS NOT A COMPOSITOR

/*
Core, kind-agnostic contract every concrete operation implements.

Value is a closed enum, not a Box<dyn Value> trait object - every
operation matches on it directly instead of downcasting, and the
payload-carrying variants hold an Arc so passing a value to N
consumers (or storing it - Ghost's history, CapturedFrame's captured
background) is a refcount bump, never a deep pixel copy.
*/

use std::any::Any;
use std::sync::Arc;

use crate::operations::{Frame, Image, Mask};
use crate::resource_manager::ResourceManager;






pub fn find_input(inputs: &[(Input, Value)], key: Input) -> Option<&Value> {
    inputs.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}






