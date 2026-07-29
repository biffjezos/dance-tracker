# DANCE TRACKER 5000 - Operation Quick Reference

## TL;DR - 7 Steps to Add a New Operation

### 1. Create File
```bash
# For transform operations
touch engine/src/operations/transform/[name].rs
```

### 2. Implement Operation Trait
```rust
pub struct MyOp { param: f64 }

impl MyOp {
    pub fn new() -> Self { Self { param: 0.0 } }
}

impl Operation for MyOp {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "my_op",
            menu: "TRANSFORM",
            label: "MY OP",
            action: None,
            ui_action: None,
            create_node: Some("my_op"),
            buttons: &[],
        }
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    
    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "My Operation",
            category: OperationCategory::Color,
            input_count: 1,
            outputs: vec![OutputKind::Image],
        }
    }
    
    fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let input = inputs.first().ok_or(OperationError::MissingInput("Need input".into()))?.1;
        match input {
            Value::Image(img) => Ok(vec![Value::Image(img.clone())]),
            _ => Err(OperationError::InvalidInputType("Need Image".into())),
        }
    }
}
```

### 3. Register with Inventory
```rust
// At bottom of file
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(MyOp::new())
    }
}
```

### 4. Export from Module
```rust
// In engine/src/operations/transform/mod.rs
pub mod my_op;
```

### 5. Add UI Edit Context (Optional)
```javascript
// In ui/scripts/engine/nodeEditContexts.js
export function renderMyOpEditContext(menuManager, nodeEntry) {
    const label = document.createElement("span");
    label.innerText = " MY OP SETTINGS ";
    menuManager.subMenu.appendChild(label);
}
nodeEditContextRegistry.register("my_op", renderMyOpEditContext);
```

### 6. Update State (Optional)
```javascript
// In ui/scripts/engine/state.js
export const state = {
    nextMyOpNumber: 1,
    myOpLayers: [],
};
```

### 7. Update Menu Creation (Optional)
```javascript
// In ui/scripts/engine/menu.js, in createNodeAndSelect:
} else if (operationId === "my_op") {
    layerId = `my_op:${state.nextMyOpNumber++}`;
    layerName = `MY OP ${state.nextMyOpNumber - 1}`;
    layerKind = "my_op";
    state.myOpLayers.push({ id: layerId, name: layerName, nodeId: nodeId, settings: {} });
}
```

---

## Key Types Summary

### OperationDescriptor Fields
| Field | Type | Purpose | Example |
|-------|------|---------|---------|
| id | `&'static str` | Unique identifier | `"blur"` |
| menu | `&'static str` | Menu category | `"TRANSFORM"` |
| label | `&'static str` | Display text | `"BLUR"` |
| action | `Option<&'static str>` | Direct action | `None` |
| ui_action | `Option<&'static str>` | UI action | `"open_image_picker"` |
| create_node | `Option<&'static str>` | Node type | `Some("blur")` |
| buttons | `&'static [OperationButton]` | Submenu | `&[]` |

### OperationMetadata Fields
| Field | Type | Purpose | Example |
|-------|------|---------|---------|
| display_name | `&'static str` | Human-readable name | `"Gaussian Blur"` |
| category | `OperationCategory` | Grouping | `OperationCategory::Color` |
| input_count | `usize` | Number of inputs | `1` |
| outputs | `Vec<OutputKind>` | Output types | `vec![OutputKind::Image]` |

### ParameterDescriptor Fields
| Field | Type | Purpose | Example |
|-------|------|---------|---------|
| name | `&'static str` | Parameter name | `"radius"` |
| kind | `ParameterKind` | Value type | `ParameterKind::Number` |

### ParameterKind Variants
- `Number` - f64 values
- `Boolean` - true/false
- `Text` - String values
- `Color` - Color values

### OperationCategory Variants
- `Source` - Input sources (image, video, camera)
- `Generator` - Content generators (rings, ghost)
- `Mask` - Mask operations
- `Composite` - Composite operations
- `Reference` - Reference operations
- `Color` - Color operations (blur, shuffle)

### OutputKind Variants
- `Frame` - Video frame
- `Mask` - Mask data
- `Image` - Static image
- `Video` - Video stream
- `Number` - Numeric value
- `Boolean` - Boolean value
- `Text` - Text string
- `Color` - Color value

### Input Variants
- `Source` - Source operations
- `Reference` - Reference inputs
- `Content` - Content to process
- `Mask` - Mask input
- `Foreground` - Foreground for composite
- `Background` - Background for composite

### Value Variants
- `Frame(Arc<Frame>)` - Video frame
- `Mask(Arc<Mask>)` - Mask
- `Image(Arc<Image>)` - Image
- `Video(Arc<Video>)` - Video
- `Number(f64)` - Number
- `Boolean(bool)` - Boolean
- `Text(String)` - Text
- `Color(Color)` - Color
- `Center(Center)` - Center point

---

## File Locations

| Purpose | Location |
|---------|----------|
| Rust operation code | `engine/src/operations/[category]/[name].rs` |
| Module exports | `engine/src/operations/[category]/mod.rs` |
| Operations module | `engine/src/operations/mod.rs` |
| Inventory system | `engine/src/operations/inventory.rs` |
| Registration | `engine/src/operations/register.rs` |
| Operation trait | `engine/src/compositor/operations.rs` |
| Descriptors | `engine/src/compositor/operation_descriptor.rs` |
| Metadata | `engine/src/compositor/metadata.rs` |
| Values | `engine/src/compositor/value.rs` |
| Inputs | `engine/src/compositor/input.rs` |
| WASM bindings | `engine/src/app.rs` |
| UI menu | `ui/scripts/engine/menu.js` |
| UI edit contexts | `ui/scripts/engine/nodeEditContexts.js` |
| UI state | `ui/scripts/engine/state.js` |
| Operations JS | `ui/scripts/core/operations.js` |

---

## Existing Operations to Study

| Operation | File | Features |
|-----------|------|----------|
| ImageSource | `operations/sources/image.rs` | Source, ui_action, no parameters |
| VideoSource | `operations/sources/video.rs` | Source, ui_action |
| CameraSource | `operations/sources/camera.rs` | Source |
| Shuffle | `operations/transform/shuffle.rs` | Parameters, edit context, enum parameters |
| ImageToFrame | `operations/converters/image_to_frame.rs` | Converter, 1 input |

---

## Common Patterns

### Simple Filter (1 input, 1 output)
```rust
fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
    let input = inputs.first().ok_or(OperationError::MissingInput("Need input".into()))?.1;
    let img = match input {
        Value::Image(i) => i,
        _ => return Err(OperationError::InvalidInputType("Need Image".into())),
    };
    let output = self.process(img);
    Ok(vec![Value::Image(Arc::new(output))])
}
```

### Composite (2 inputs)
```rust
fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
    let fg = find_input(inputs, Input::Foreground).ok_or(OperationError::MissingInput("Need FG".into()))?;
    let bg = find_input(inputs, Input::Background).ok_or(OperationError::MissingInput("Need BG".into()))?;
    let fg_img = match fg { Value::Image(i) => i, _ => return Err(...) };
    let bg_img = match bg { Value::Image(i) => i, _ => return Err(...) };
    let output = self.composite(fg_img, bg_img);
    Ok(vec![Value::Image(Arc::new(output))])
}
```

### With Parameters
```rust
fn parameters(&self) -> Vec<ParameterDescriptor> {
    vec![
        ParameterDescriptor { name: "radius", kind: ParameterKind::Number },
        ParameterDescriptor { name: "enabled", kind: ParameterKind::Boolean },
    ]
}

fn get_parameter(&self, name: &str) -> Option<Value> {
    match name {
        "radius" => Some(Value::Number(self.radius)),
        "enabled" => Some(Value::Boolean(self.enabled)),
        _ => None,
    }
}

fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
    match (name, value) {
        ("radius", Value::Number(v)) => { self.radius = v; Ok(()) }
        ("enabled", Value::Boolean(v)) => { self.enabled = v; Ok(()) }
        _ => Err(OperationError::UnknownParameter(name.into())),
    }
}
```

---

## Inventory System Flow

```
[Your Operation File]
    inventory::submit! { OperationInfo { constructor } }
        ↓
[Compile Time - inventory crate collects all submissions]
        ↓
[operations/inventory.rs - initialize_inventory()]
    → Creates Vec<RegisteredOperationInfo>
        ↓
[operations/register.rs - register_operations()]
    → Calls registry.register_from_inventory()
        ↓
[compositor/registry.rs - register_from_inventory()]
    → Calls inventory::populate_registry()
        ↓
[app.rs - App::new()]
    → Calls register::register_operations()
        ↓
[Runtime - Operations available in registry]
```

---

## UI Integration Flow

```
[app.rs - get_operations()]
    → Returns serde_wasm_bindgen::to_value(&registry.descriptors())
        ↓
[ui/scripts/core/operations.js - getOperations()]
    → Receives operation descriptors
    → Builds operationMap
        ↓
[ui/scripts/engine/menu.js - MenuManager]
    → Filters operations by menu category
    → Creates buttons for each operation
    → Handles create_node flow
        ↓
[User clicks operation button]
    → Triggers createNode(operationId)
        ↓
[app.rs - create_node()]
    → Creates operation from registry
    → Adds to graph
    → Returns node_id
        ↓
[ui/scripts/engine/menu.js - createNodeAndSelect()]
    → Creates layer entry
    → Adds to state
    → Selects new node
```

---

## Debugging Tips

### Check if operation is registered
```javascript
// In browser console
wasmApp.get_operations()
// Should include your operation
```

### Check inventory
```rust
// In Rust code, temporarily add:
#[wasm_bindgen]
pub fn debug_operations() -> JsValue {
    serde_wasm_bindgen::to_value(&crate::operations::inventory::get_all_descriptors())
        .unwrap()
}
```

### Verify module exports
```bash
# Check if your module is exported
cargo doc --no-deps --open
# Navigate to operations module
```

---

## Gotchas

1. **Static strings**: All descriptor strings must be `&'static str` (string literals)
2. **Inventory submission**: Must be at module level, not inside a function
3. **Module exports**: Parent module must `pub mod` your operation file
4. **Type matching**: Input types must match what's passed from the graph
5. **Arc usage**: Images, Frames, etc. must be wrapped in `Arc` for return values
6. **Error handling**: Always validate inputs and return proper errors
7. **Parameter names**: Must match between `parameters()`, `get_parameter()`, and `set_parameter()`
