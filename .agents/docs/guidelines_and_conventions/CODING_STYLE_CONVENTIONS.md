---
owner_role: "management"
token_count: 482
---
# Coding Style Conventions

This file should be used by all agent roles and active agents.

Apply these Coding Style Conventions to all new code.
Change existing code to follow Coding Style Conventions if edited.

## Line-Breaks

### General

- No line-breaks before approaching 120 characters per line, if not invalidated by a following
convention.

### Small arrays, enums and functions

Put small arrays, enums with only one, two or three entries in one line.

**Bad**
```rust
kind: ParameterKind::Enum(&[
    "CPU",
    "GPU",
    "AUTO",
]),
``` 
**Good**
```rust
kind: ParameterKind::Enum(&["CPU", "GPU","AUTO", ]),
``` 

### Single-line function signature

**Bad**
```rust
pub fn find_input(
    inputs: &[(Input, Value)],
    key: Input
    ) -> Option<&Value> {
    ..
}
```
**Good**
```rust
pub fn find_input(inputs: &[(Input, Value)], key: Input) -> Option<&Value> {
    ..
}
```

## Imports

In rust, use nested use-statements. Do not add a second statement for a higher module.
Keep a space after opening brackets and before closing brackets.

**Bad**
```rust
use crate::compositor::{bbox::Rect, Context, Input, OperationError, Value};
use crate::compositor::graph::{Graph, NodeId};
```

**Good**
```rust
use crate::compositor::{
    bbox::{ Rect, Context, Input, OperationError, Value },
    graph::{ Graph, NodeId }
};
```

## Final-Comma

Do not add last comma into struct definitions, enums, match-arms.

**Bad**
```rust
OperationDescriptor {
    ..
    create_node: None,
    submenu: None,
}
```
**Good**
```rust
OperationDescriptor {
    ..
    create_node: None,
    submenu: None
}
```

## Closing-')' in chaining functions

Put the closing ')' on the new line.

**Bad**
```rust
self.operations
    .iter()
    .find(|op| op.descriptor.id == id)
    .map(|op| (op.constructor)())
```
**Good**
```rust
self.operations
    .iter()
    .find(|op| op.descriptor.id == id)
    .map(|op| (op.constructor)()
)
```

# Comments

Write precise and concise comments. Do not overexplain. Add a very brief rationale only. State if the code is currently unused but needed for the future. Flag dead code.