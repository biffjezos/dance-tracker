# DANCE TRACKER 5000 - Operation Implementation Guide

## Overview

This guide provides a complete, step-by-step instruction set for implementing new operations (such as BLUR, COMPOSITE, etc.) in the DANCE TRACKER 5000 engine. The architecture is designed around a flexible, inventory-based registration system that automatically discovers and registers operations at compile time.

---

## Architecture Overview

The system consists of several key components:

1. **Operation Trait** (`compositor/operations.rs`) - Defines the interface all operations must implement
2. **OperationDescriptor** (`compositor/operation_descriptor.rs`) - Metadata for menu integration and UI
3. **OperationMetadata** (`compositor/metadata.rs`) - Technical metadata about the operation
4. **Inventory System** (`operations/inventory.rs`) - Compile-time operation discovery
5. **OperationRegistry** (`compositor/registry.rs`) - Runtime operation management
6. **Value Enum** (`compositor/value.rs`) - Data types that flow through the graph
7. **Input Enum** (`compositor/input.rs`) - Input socket types for operations
8. **WASM Bindings** (`app.rs`) - JavaScript interop layer
9. **UI Integration** (`ui/scripts/...`) - Menu system and node editing

---

## Step 1: Create the Operation Rust File

Create a new Rust file for your operation in the appropriate module directory:
- **Sources** (image, video, camera): `engine/src/operations/sources/`
- **Transforms** (blur, shuffle): `engine/src/operations/transform/`
- **Converters**: `engine/src/operations/converters/`
- **New categories**: Create a new subdirectory under `engine/src/operations/`

### File Structure Template

```rust
// engine/src/operations/[category]/[operation_name].rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind },
    Value
};
use crate::graphics::{Image, ImageFormat, Frame, Mask};

/// Your operation struct
/// Contains all state and parameters for the operation
pub struct [OperationName] {
    // Define your operation's parameters here
    pub parameter1: f64,
    pub parameter2: bool,
    // ...
}

impl [OperationName] {
    /// Constructor - creates a new instance with default values
    pub fn new() -> Self {
        Self {
            parameter1: 0.0,  // Default value
            parameter2: false,
        }
    }
    
    // Add any helper methods here
    // These are not part of the Operation trait but can be used internally
}
```

---

## Step 2: Implement the Operation Trait

Every operation MUST implement the `Operation` trait. This is the core contract.

### Required Trait Methods

```rust
impl Operation for [OperationName] {
    /// DESCRIPTOR: Defines how the operation appears in the UI menu system
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "[unique_lowercase_id]",           // e.g., "blur", "composite", "image_source"
            menu: "[MENU_CATEGORY]",               // e.g., "INPUT", "TRANSFORM", "COMPOSITE"
            label: "[DISPLAY_LABEL]",              // e.g., "BLUR", "COMPOSITE", "LOAD IMAGE"
            action: None,                          // Optional: direct action to trigger
            ui_action: None,                       // Optional: UI-specific action (e.g., "open_image_picker")
            create_node: Some("[node_type]"),     // Optional: creates a graph node when selected
            buttons: &[],                          // Optional: submenu buttons
        }
    }
    
    /// AS_ANY: Enable downcasting for type-specific operations
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    /// AS_ANY_MUT: Enable mutable downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    /// METADATA: Technical information about the operation
    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "[Human-readable name]",  // e.g., "Gaussian Blur"
            category: OperationCategory::[Category], // Source, Generator, Mask, Composite, Reference, Color
            input_count: [number_of_inputs],        // 0 for sources, 1 for filters, 2+ for compositors
            outputs: vec![OutputKind::[Type]],       // Frame, Mask, Image, Video, Number, Boolean, Text, Color
        }
    }
    
    /// EXECUTE: The core processing function
    /// Takes context and inputs, returns output values
    fn execute(
        &self,
        ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        // 1. Validate inputs
        if inputs.is_empty() {
            return Err(OperationError::MissingInput("Operation requires input".to_string()));
        }
        
        // 2. Extract input values
        let input_value = inputs.first().unwrap().1.clone();
        
        // 3. Match on input type and process
        let input_image = match input_value {
            Value::Image(img) => img,
            Value::Frame(frame) => {
                // Convert frame to image if needed
                // ...
                return Err(OperationError::InvalidInputType("Expected Image".to_string()));
            }
            _ => return Err(OperationError::InvalidInputType("Expected Image".to_string())),
        };
        
        // 4. Process the data
        // Your algorithm implementation here
        
        // 5. Return output values
        Ok(vec![Value::Image(output_image)])
    }
}
```

### Optional Trait Methods (for Parameterized Operations)

If your operation has configurable parameters, implement these methods:

```rust
impl Operation for [OperationName] {
    // ... previous methods ...
    
    /// PARAMETERS: Define what parameters this operation exposes
    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "radius",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "enabled",
                kind: ParameterKind::Boolean,
            },
        ]
    }
    
    /// GET_PARAMETER: Return the current value of a parameter
    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "radius" => Some(Value::Number(self.radius)),
            "enabled" => Some(Value::Boolean(self.enabled)),
            _ => None,
        }
    }
    
    /// SET_PARAMETER: Update a parameter value
    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("radius", Value::Number(v)) => {
                self.radius = v;
                Ok(())
            }
            ("enabled", Value::Boolean(v)) => {
                self.enabled = v;
                Ok(())
            }
            (name, _) => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }
    
    /// SUPPORTS_EDIT: Returns true if this operation has editable parameters
    fn supports_edit(&self) -> bool {
        !self.parameters().is_empty()
    }
}
```

---

## Step 3: Register with Inventory System

At the bottom of your operation file, add the inventory submission:

```rust
// Inventory registration for [OperationName]
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new([OperationName]::new())
    }
}
```

This is the **critical** step that makes your operation discoverable by the system. The `inventory` crate collects all submissions at compile time, and the `register_operations` function in `operations/register.rs` populates the registry from this inventory.

---

## Step 4: Update Module Exports

Ensure your new operation is exported from its parent module.

### For new files in existing modules:

Edit the module file (e.g., `engine/src/operations/transform/mod.rs`):

```rust
// engine/src/operations/transform/mod.rs
pub mod shuffle;
pub mod [operation_name];  // Add this line
```

### For new modules:

Create a new module file and export it from `engine/src/operations/mod.rs`:

```rust
// engine/src/operations/mod.rs
pub mod converters;
pub mod inventory;
pub mod register;
pub mod sources;
pub mod transform;
pub mod [new_module];  // Add this line
```

---

## Step 5: UI Integration (JavaScript)

### Menu System Integration

The menu system in `ui/scripts/engine/menu.js` automatically discovers operations from the WASM bindings. Your operation's `descriptor()` defines how it appears in the menu:

- **`menu` field**: Determines which top-level menu category it appears under
- **`label` field**: The button text in the menu
- **`create_node` field**: If set, creates a graph node when selected
- **`ui_action` field**: If set, triggers a UI-specific action (like file picker)
- **`buttons` field**: If set, shows a submenu with these buttons

### Node Edit Context (Optional)

If your operation supports editing, register an edit context in `ui/scripts/engine/nodeEditContexts.js`:

```javascript
// Add to nodeEditContexts.js

/**
 * Render function for [OPERATION] edit context
 * Shows: [parameter controls]
 */
export function render[OperationName]EditContext(menuManager, nodeEntry) {
    // Create UI controls for your operation's parameters
    const label = document.createElement("span");
    label.innerText = ` ${nodeEntry.label} SETTINGS `;
    label.className = "node-selector-label";
    menuManager.subMenu.appendChild(label);
    
    // Add parameter controls here
    // Example: slider for radius
    const radiusLabel = document.createElement("span");
    radiusLabel.innerText = " RADIUS ";
    menuManager.subMenu.appendChild(radiusLabel);
    
    // ... more controls
}

// Register the context
nodeEditContextRegistry.register("[operation_id]", render[OperationName]EditContext);
```

### WASM Binding Updates (Optional)

If your operation needs special handling in the WASM bindings (like the Shuffle operation's parameter updates), add methods to `app.rs`:

```rust
// In app.rs

/// Update a parameter on a specific [OperationName] node
pub fn update_[operation_name]_parameter(
    &mut self,
    node_id: u32,
    parameter: String,
    value: String,
) -> Result<(), JsValue> {
    let node_id = NodeId::from_index(node_id);
    
    let operation = self.graph.get_node_mut(&node_id)
        .ok_or_else(|| JsValue::from_str(&format!("Node {:?} not found", node_id)))?;
    
    if let Some(op) = operation.as_any_mut().downcast_mut::<[OperationName]>() {
        // Handle parameter updates
        match parameter.as_str() {
            "param1" => {
                op.param1 = parse_value(&value)?;
                return Ok(());
            }
            _ => return Err(JsValue::from_str(&format!("Unknown parameter: {}", parameter))),
        };
    }
    
    Err(JsValue::from_str(&format!("Node {:?} is not a [OperationName]", node_id)))
}
```

---

## Step 6: State Management (Optional)

If your operation creates nodes that need to be tracked in the UI state, update `ui/scripts/engine/state.js`:

```javascript
// In state.js
export const state = {
    // ... existing state
    next[OperationName]Number: 1,  // For generating unique IDs
    [operationName]Layers: [],      // Array to track instances
};
```

And update the `createNodeAndSelect` method in `menu.js` to handle your operation type.

---

## Complete Example: BLUR Operation

Here's a complete example implementing a Gaussian Blur operation:

### Rust Implementation

```rust
// engine/src/operations/transform/blur.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind },
    Value
};
use crate::graphics::{Image, ImageFormat};

/// Gaussian Blur operation
pub struct Blur {
    pub radius: f64,
}

impl Blur {
    pub fn new() -> Self {
        Self {
            radius: 1.0,
        }
    }
    
    /// Apply Gaussian blur to the input image
    fn apply_blur(&self, input: &Image) -> Image {
        // Simplified blur implementation
        // In production, use a proper Gaussian kernel
        let width = input.width;
        let height = input.height;
        let mut output_pixels = vec![0u8; (width * height * 4) as usize];
        
        let radius_int = self.radius.max(1.0) as usize;
        
        for y in 0..height as usize {
            for x in 0..width as usize {
                let mut r_sum = 0u32;
                let mut g_sum = 0u32;
                let mut b_sum = 0u32;
                let mut a_sum = 0u32;
                let mut count = 0usize;
                
                // Simple box blur for demonstration
                for dy in 0..=radius_int {
                    for dx in 0..=radius_int {
                        let nx = x.saturating_sub(dx).min(width as usize - 1);
                        let ny = y.saturating_sub(dy).min(height as usize - 1);
                        let index = (ny * width as usize + nx) * 4;
                        
                        if index + 3 < input.pixels.len() {
                            r_sum += input.pixels[index] as u32;
                            g_sum += input.pixels[index + 1] as u32;
                            b_sum += input.pixels[index + 2] as u32;
                            a_sum += input.pixels[index + 3] as u32;
                            count += 1;
                        }
                    }
                }
                
                let output_index = (y * width as usize + x) * 4;
                output_pixels[output_index] = (r_sum / count as u32) as u8;
                output_pixels[output_index + 1] = (g_sum / count as u32) as u8;
                output_pixels[output_index + 2] = (b_sum / count as u32) as u8;
                output_pixels[output_index + 3] = (a_sum / count as u32) as u8;
            }
        }
        
        Image {
            pixels: output_pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        }
    }
}

impl Operation for Blur {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "blur",
            menu: "TRANSFORM",
            label: "BLUR",
            action: None,
            ui_action: None,
            create_node: Some("blur"),
            buttons: &[],
        }
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Gaussian Blur",
            category: OperationCategory::Color,
            input_count: 1,
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "radius",
                kind: ParameterKind::Number,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "radius" => Some(Value::Number(self.radius)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("radius", Value::Number(v)) => {
                self.radius = v.max(0.0);
                Ok(())
            }
            (name, _) => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let input_value = inputs.first()
            .ok_or_else(|| OperationError::MissingInput("Blur requires an input".to_string()))?
            .1.clone();

        let input_image = match input_value {
            Value::Image(img) => img,
            _ => return Err(OperationError::InvalidInputType("Blur requires an Image input".to_string())),
        };

        let output_image = Arc::new(self.apply_blur(&input_image));
        
        Ok(vec![Value::Image(output_image)])
    }
}

// Inventory registration for Blur
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Blur::new())
    }
}
```

### Module Export

```rust
// engine/src/operations/transform/mod.rs
pub mod shuffle;
pub mod blur;  // Add this line
```

### JavaScript UI Integration

```javascript
// In ui/scripts/engine/nodeEditContexts.js

/**
 * Render function for BLUR edit context
 * Shows: BLUR RADIUS control
 */
export function renderBlurEditContext(menuManager, nodeEntry) {
    const blurLabel = document.createElement("span");
    blurLabel.innerText = " BLUR SETTINGS ";
    blurLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(blurLabel);
    
    // Add radius control
    const radiusLabel = document.createElement("span");
    radiusLabel.innerText = " RADIUS ";
    radiusLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(radiusLabel);
    
    // In a real implementation, you'd add a slider or input field here
    // that dispatches updateNodeParameter events
}

// Register the context (add to existing registrations)
nodeEditContextRegistry.register("blur", renderBlurEditContext);
```

### State Management

```javascript
// In ui/scripts/engine/state.js
export const state = {
    // ... existing state
    nextBlurNumber: 1,
    blurLayers: [],
};
```

```javascript
// In ui/scripts/engine/menu.js, update createNodeAndSelect
// Add a new branch for blur:
} else if (operationId === "blur") {
    layerId = `blur:${state.nextBlurNumber++}`;
    layerName = `BLUR ${state.nextBlurNumber - 1}`;
    layerKind = "blur";
    
    const newLayer = {
        id: layerId,
        name: layerName,
        nodeId: nodeId,
        settings: { radius: 1.0 }
    };
    state.blurLayers.push(newLayer);
}
```

---

## Step 7: COMPOSITE Operation Example

For operations that combine multiple inputs (like composite), the pattern is similar but with multiple inputs:

```rust
// engine/src/operations/composite/mod.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind },
    Value
};
use crate::graphics::{Image, ImageFormat};

/// Composite operation - combines foreground and background
pub struct Composite {
    pub opacity: f64,
    pub blend_mode: BlendMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendMode {
    Normal,
    Add,
    Multiply,
    Screen,
    Overlay,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::Normal
    }
}

impl Composite {
    pub fn new() -> Self {
        Self {
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
        }
    }
    
    fn blend_pixels(&self, fg: &[u8], bg: &[u8]) -> Vec<u8> {
        // Implement blending based on mode and opacity
        // ...
        vec![]
    }
}

impl Operation for Composite {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "composite",
            menu: "COMPOSITE",
            label: "COMPOSITE",
            action: None,
            ui_action: None,
            create_node: Some("composite"),
            buttons: &[],
        }
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Composite",
            category: OperationCategory::Composite,
            input_count: 2,  // Foreground and Background
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "opacity",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "blend_mode",
                kind: ParameterKind::Text,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "opacity" => Some(Value::Number(self.opacity)),
            "blend_mode" => Some(Value::Text(format!("{:?}", self.blend_mode))),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("opacity", Value::Number(v)) => {
                self.opacity = v.clamp(0.0, 1.0);
                Ok(())
            }
            ("blend_mode", Value::Text(s)) => {
                self.blend_mode = match s.as_str() {
                    "Normal" => BlendMode::Normal,
                    "Add" => BlendMode::Add,
                    "Multiply" => BlendMode::Multiply,
                    "Screen" => BlendMode::Screen,
                    "Overlay" => BlendMode::Overlay,
                    _ => return Err(OperationError::InvalidParameterValue(name.to_string(), s)),
                };
                Ok(())
            }
            (name, _) => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        // Find foreground and background inputs
        let fg_value = crate::compositor::input::find_input(inputs, Input::Foreground)
            .ok_or_else(|| OperationError::MissingInput("Composite requires foreground input".to_string()))?;
        let bg_value = crate::compositor::input::find_input(inputs, Input::Background)
            .ok_or_else(|| OperationError::MissingInput("Composite requires background input".to_string()))?;

        let fg_image = match fg_value {
            Value::Image(img) => img,
            _ => return Err(OperationError::InvalidInputType("Foreground must be an Image".to_string())),
        };
        let bg_image = match bg_value {
            Value::Image(img) => img,
            _ => return Err(OperationError::InvalidInputType("Background must be an Image".to_string())),
        };

        // Ensure images are the same size
        if fg_image.width != bg_image.width || fg_image.height != bg_image.height {
            return Err(OperationError::DimensionMismatch);
        }

        // Blend the images
        let output_pixels = self.blend_pixels(&fg_image.pixels, &bg_image.pixels);
        
        let output_image = Arc::new(Image {
            pixels: output_pixels,
            width: fg_image.width,
            height: fg_image.height,
            format: ImageFormat::Rgba8,
        });

        Ok(vec![Value::Image(output_image)])
    }
}

// Inventory registration
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Composite::new())
    }
}
```

---

## Step 8: Testing Your Operation

1. **Compile the Rust code**:
   ```bash
   cd engine
   cargo build --target wasm32-unknown-unknown
   ```

2. **Check for compilation errors** and fix any issues.

3. **Test in the browser**:
   - Load the application in a browser
   - Navigate to your operation's menu category
   - Verify the operation appears in the menu
   - Create a node and test its functionality
   - If applicable, test the edit context

---

## Common Patterns and Best Practices

### 1. Input Handling

Always validate inputs in the `execute` method:

```rust
fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
    // Check minimum input count
    if inputs.len() < self.metadata().input_count {
        return Err(OperationError::MissingInput(
            format!("Requires {} inputs", self.metadata().input_count)
        ));
    }
    
    // Find specific inputs
    let input_value = crate::compositor::input::find_input(inputs, Input::Content)
        .ok_or_else(|| OperationError::MissingInput("Requires Content input".to_string()))?;
    
    // Match on input type
    match input_value {
        Value::Image(img) => { /* process image */ }
        Value::Frame(frame) => { /* process frame */ }
        Value::Mask(mask) => { /* process mask */ }
        _ => return Err(OperationError::InvalidInputType("Invalid input type".to_string())),
    }
    
    // ... processing ...
}
```

### 2. Parameter Validation

Validate parameter values in `set_parameter`:

```rust
fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
    match (name, value) {
        ("radius", Value::Number(v)) => {
            if v < 0.0 {
                return Err(OperationError::InvalidParameterValue(
                    name.to_string(),
                    format!("Radius must be non-negative, got {}", v)
                ));
            }
            self.radius = v;
            Ok(())
        }
        ("enabled", Value::Boolean(v)) => {
            self.enabled = v;
            Ok(())
        }
        (name, _) => Err(OperationError::UnknownParameter(name.to_string())),
    }
}
```

### 3. Image Processing Helpers

Create helper methods for common image operations:

```rust
impl Blur {
    /// Get pixel at (x, y) with bounds checking
    fn get_pixel(&self, image: &Image, x: usize, y: usize) -> (u8, u8, u8, u8) {
        let x = x.min(image.width as usize - 1);
        let y = y.min(image.height as usize - 1);
        let index = (y * image.width as usize + x) * 4;
        (
            image.pixels[index],
            image.pixels[index + 1],
            image.pixels[index + 2],
            image.pixels[index + 3],
        )
    }
    
    /// Set pixel at (x, y)
    fn set_pixel(output: &mut [u8], x: usize, y: usize, width: u32, r: u8, g: u8, b: u8, a: u8) {
        let index = (y * width as usize + x) * 4;
        output[index] = r;
        output[index + 1] = g;
        output[index + 2] = b;
        output[index + 3] = a;
    }
}
```

### 4. Working with Frames

If your operation needs to work with Frames instead of Images:

```rust
fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
    let input_value = inputs.first().unwrap().1.clone();
    
    let frame = match input_value {
        Value::Frame(f) => f,
        Value::Image(img) => {
            // Convert Image to Frame if needed
            Arc::new(Frame {
                pixels: img.pixels.clone(),
                width: img.width,
                height: img.height,
                timestamp: ctx.meta.time,
            })
        }
        _ => return Err(OperationError::InvalidInputType("Expected Frame or Image".to_string())),
    };
    
    // Process frame
    // ...
    
    Ok(vec![Value::Frame(output_frame)])
}
```

### 5. Context Usage

The `Context` parameter provides access to:
- Frame counter
- FPS
- Time
- Preview mode flag
- Render quality
- Resolution
- Resource manager

```rust
fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
    // Access context information
    let frame_number = ctx.meta.frame;
    let time = ctx.meta.time;
    let is_preview = ctx.meta.preview;
    let (width, height) = (ctx.meta.width, ctx.meta.height);
    
    // Use resources
    // let resource = ctx.resources.get("some_resource");
    
    // ...
}
```

---

## Input Types Reference

The `Input` enum defines the types of inputs an operation can receive:

```rust
pub enum Input {
    Source,      // Source operations (image, video, camera)
    Reference,   // Reference inputs
    Content,     // Content to process
    Mask,        // Mask for masking operations
    Foreground,  // Foreground for composite operations
    Background,  // Background for composite operations
}
```

Use `crate::compositor::input::find_input(inputs, Input::[Type])` to extract specific inputs.

---

## Output Types Reference

The `OutputKind` enum defines what an operation can produce:

```rust
pub enum OutputKind {
    Frame,    // Video frame with timestamp
    Mask,     // Mask data
    Image,    // Static image
    Video,    // Video stream
    Number,   // Numeric value
    Boolean,  // Boolean value
    Text,     // Text string
    Color,    // Color value
}
```

Return values using the `Value` enum:

```rust
pub enum Value {
    Frame(Arc<Frame>),
    Mask(Arc<Mask>),
    Image(Arc<Image>),
    Video(Arc<Video>),
    Number(f64),
    Boolean(bool),
    Text(String),
    Color(Color),
    Center(Center),
}
```

---

## Operation Categories Reference

```rust
pub enum OperationCategory {
    Source,      // Source operations (image, video, camera)
    Generator,   // Generates content (rings, ghost, etc.)
    Mask,        // Mask operations
    Composite,   // Composite operations (combine inputs)
    Reference,   // Reference operations
    Color,       // Color operations (shuffle, blur, etc.)
}
```

---

## Error Handling Reference

```rust
pub enum OperationError {
    MissingInput(String),
    WrongValueType,
    DimensionMismatch,
    SourceNotFound(String),
    Cycle(Vec<usize>),
    UnknownParameter(String),
    UnknownNode,
    NotImplemented(String),
    InvalidInputType(String),
    InvalidParameterType(String),
    InvalidParameterValue(String, String),
}
```

---

## Checklist for New Operations

- [ ] Created Rust file in appropriate module directory
- [ ] Implemented `Operation` trait
- [ ] Defined `descriptor()` with correct menu, label, and create_node
- [ ] Defined `metadata()` with correct category, input_count, and outputs
- [ ] Implemented `execute()` method with proper input validation
- [ ] Added parameter support if needed (`parameters()`, `get_parameter()`, `set_parameter()`)
- [ ] Registered with inventory system (`inventory::submit!`)
- [ ] Exported from parent module
- [ ] Added UI edit context (if editable)
- [ ] Updated state management (if needed)
- [ ] Tested compilation
- [ ] Tested in browser

---

## Troubleshooting

### Operation not appearing in menu
1. Check that `inventory::submit!` is at the bottom of your file
2. Verify the module is properly exported
3. Check that `register_operations` is called in `app.rs`
4. Verify the descriptor's `menu` field matches an existing menu category

### Operation creates node but doesn't work
1. Check that `create_node` field in descriptor is set
2. Verify the operation ID matches what's expected in the UI
3. Check that the `execute` method handles inputs correctly
4. Verify input types match what's being passed

### Parameters not editable
1. Check that `parameters()` returns non-empty list
2. Verify `supports_edit()` returns true
3. Ensure UI edit context is registered
4. Check that `get_parameter()` and `set_parameter()` are implemented

### Compilation errors
1. Check all imports are correct
2. Verify trait implementations are complete
3. Check that all types are in scope
4. Ensure proper use of `Arc` for shared data

---

## Summary

The DANCE TRACKER 5000 operation system is designed for flexibility and extensibility. By following this guide, you can implement new operations that seamlessly integrate with the existing architecture. The key principles are:

1. **Implement the Operation trait** - This is the core contract
2. **Use the inventory system** - For automatic registration
3. **Define proper metadata** - For UI integration
4. **Handle inputs and outputs correctly** - Using the Value and Input enums
5. **Support parameters if needed** - For editable operations
6. **Integrate with UI** - Through descriptors and edit contexts

The system automatically discovers operations at compile time, registers them in the menu system, and handles their execution in the graph. Your main job is to implement the processing logic in the `execute` method and define the operation's interface through its descriptor and metadata.
