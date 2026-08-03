
use crate::compositor::input::Input;
use serde::Serialize;
/*
Only the scalar Value variants make sense as something a UI would show
a control for - Frame/Mask/Image are graph-wired inputs, never a
setting on the node that produces them.
*/
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParameterKind {
    /// A stepper's increment and, optionally, its bounds - the operation
    /// owns them, so a UI stepper moves by exactly this amount and cannot
    /// be pushed past a bound the operation itself would reject.
    Number { step: f64, min: Option<f64>, max: Option<f64> },
    Boolean,
    Text,
    Color,
    /// A closed set of values. The operation owns the list, so a UI selector
    /// offers exactly these and nothing else.
    Enum(&'static [&'static str])
}

impl ParameterKind {
    /// The values a selector may offer for this parameter - empty for
    /// parameters that are not a closed set.
    pub fn options(&self) -> &'static [&'static str] {
        match self {
            ParameterKind::Enum(options) => options,
            _ => &[],
        }
    }

    /// The stepper increment for a Number parameter - None for every other kind.
    pub fn step(&self) -> Option<f64> {
        match self {
            ParameterKind::Number { step, .. } => Some(*step),
            _ => None,
        }
    }

    /// The lower bound a Number parameter accepts, if any.
    pub fn min(&self) -> Option<f64> {
        match self {
            ParameterKind::Number { min, .. } => *min,
            _ => None,
        }
    }

    /// The upper bound a Number parameter accepts, if any.
    pub fn max(&self) -> Option<f64> {
        match self {
            ParameterKind::Number { max, .. } => *max,
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ParameterKind::Number { .. } => "NUMBER",
            ParameterKind::Boolean => "BOOLEAN",
            ParameterKind::Text => "TEXT",
            ParameterKind::Color => "COLOR",
            ParameterKind::Enum(_) => "ENUM",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ParameterDescriptor {
    pub name: &'static str,
    pub kind: ParameterKind,
    /// Which named sub-pane this parameter belongs to in the editor, if
    /// any (e.g. "DIMENSION", "COLOUR"). Ungrouped (None) parameters
    /// render at the top level - the operation owns the grouping, the UI
    /// only ever renders the distinct group names it's given.
    pub group: Option<&'static str>,
}

/*
What kind of thing an operation is, for grouping in a future automatic
node menu/editor - Reference covers CapturedFrame, a settable handle
rather than something that decodes, generates, keys, or composites.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationCategory {
    Source,
    Generator,
    Mask,
    Composite,
    Reference,
    Color,
    // Produces a Number (or several), never pixels - distinct from
    // Generator, which always means "produces an Image".
    Animation,
}

impl OperationCategory {
    /// JS-facing tag - lets a future generic menu/list grouping switch on
    /// this instead of a hand-maintained per-menu category string.
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationCategory::Source => "source",
            OperationCategory::Generator => "generator",
            OperationCategory::Mask => "mask",
            OperationCategory::Composite => "composite",
            OperationCategory::Reference => "reference",
            OperationCategory::Color => "color",
            OperationCategory::Animation => "animation",
        }
    }
}

/*
Which Value variant(s) an operation's execute() can return - every
operation here only ever produces exactly one output today, but this
is a Vec (not a single OutputKind) so a future multi-output operation
doesn't need the shape to change.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Frame,
    Mask,
    // Bounded (u8) - what a source (ImageSource/CameraSource/VideoSource)
    // naturally produces, and what CLAMP converts back to.
    Image,
    // Unbounded RGBA - see graphics::FloatImage. What every other
    // operation works in and declares by default.
    FloatImage,
    Video,
    Number,
    Boolean,
    Text,
    Color
}

impl OutputKind {
    /// JS-facing tag, same convention as `OperationCategory::as_str()` -
    /// never `#[derive(Serialize)]` on the enum itself.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputKind::Frame => "frame",
            OutputKind::Mask => "mask",
            OutputKind::Image => "image",
            OutputKind::FloatImage => "float_image",
            OutputKind::Video => "video",
            OutputKind::Number => "number",
            OutputKind::Boolean => "boolean",
            OutputKind::Text => "text",
            OutputKind::Color => "color",
        }
    }
}

/*
Which OutputKinds may be wired into a given input slot. Empty = no
restriction (every real node is a valid candidate) - the escape hatch
PATCH's SOURCE needs, and exactly today's pre-typed behavior, so it's the
correct interim state for anything not yet migrated. `accepts` lives per
operation (not per `Input` variant) because the same `Input` slot name
means different things on different operations - e.g. `Reference` is a
Number source on PATCH but a pixel source on HueKey.
*/
#[derive(Clone, Debug)]
pub struct InputDescriptor {
    pub kind: Input,
    pub accepts: &'static [OutputKind],
}

/// Every pixel-producing OutputKind - the common `accepts` list for any
/// input slot that expects pixel data (image/mask/video-like content).
pub const PIXEL_KINDS: &[OutputKind] = &[
    OutputKind::Frame,
    OutputKind::Mask,
    OutputKind::Image,
    OutputKind::FloatImage,
    OutputKind::Video,
];

#[derive(Clone, Debug)]
pub struct OperationMetadata {
    pub display_name: &'static str,
    pub category: OperationCategory,
    /// Which inputs this operation can be wired to, in menu order, and what
    /// OutputKinds each accepts. A source operation declares none; anything
    /// the UI offers to wire comes from here, never from a hardcoded list on
    /// the UI side.
    pub inputs: Vec<InputDescriptor>,
    pub outputs: Vec<OutputKind>,
}

impl OperationMetadata {
    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_kind_as_str_matches_the_js_facing_contract() {
        assert_eq!(OutputKind::Frame.as_str(), "frame");
        assert_eq!(OutputKind::Mask.as_str(), "mask");
        assert_eq!(OutputKind::Image.as_str(), "image");
        assert_eq!(OutputKind::FloatImage.as_str(), "float_image");
        assert_eq!(OutputKind::Video.as_str(), "video");
        assert_eq!(OutputKind::Number.as_str(), "number");
        assert_eq!(OutputKind::Boolean.as_str(), "boolean");
        assert_eq!(OutputKind::Text.as_str(), "text");
        assert_eq!(OutputKind::Color.as_str(), "color");
    }
}
