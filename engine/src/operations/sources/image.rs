// src/operations/sources/image.rs
use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind },
    Value
};

use crate::graphics::U8Image;

pub struct ImageSource {
    pub image: Option<Arc<U8Image>>,
    // The loaded image contain-fitted to whatever resolution it was last
    // asked for, cached by that resolution - so a graph resolution that
    // hasn't changed since last tick reuses the exact same Arc (RefCell,
    // not recomputed inside execute(&self, ..)), keeping the
    // frame-to-frame cache's pointer-identity comparison meaningful for
    // every node downstream of this one.
    fitted: RefCell<Option<(u32, u32, Arc<U8Image>)>>,
}

impl ImageSource {
    pub fn new() -> Self {
        Self {
            image: None,
            fitted: RefCell::new(None),
        }
    }

    pub fn set_image(&mut self, image: Arc<U8Image>) {
        self.image = Some(image);
        *self.fitted.borrow_mut() = None;
    }

    pub fn get_image(&self) -> Option<Arc<U8Image>> {
        self.image.clone()
    }
}

impl Operation for ImageSource {
    
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "image_source",
            menu: "INPUT",
            label: "LOAD IMAGE",
            action: None,
            ui_action: Some("open_image_picker"),
            create_node: None,
            submenu: None,
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
            display_name: "Image Source",
            category: OperationCategory::Source,
            inputs: Vec::new(),
            outputs: vec![OutputKind::Image],
        }
    }


    fn execute(&self, ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let image = self
            .image
            .clone()
            .ok_or_else(|| OperationError::SourceNotFound("Image not loaded".to_string()))?;

        let (width, height) = (ctx.meta.width, ctx.meta.height);

        if let Some((cached_width, cached_height, fitted)) = self.fitted.borrow().as_ref() {
            if *cached_width == width && *cached_height == height {
                return Ok(vec![Value::Image(fitted.clone())]);
            }
        }

        // Conform to the graph's current resolution the same way a live
        // video/camera source already does, so compositing a loaded image
        // against a video never hits a dimension mismatch just because the
        // image file's own native size doesn't match the canvas.
        let fitted = Arc::new(image.contain_fit(width, height));
        *self.fitted.borrow_mut() = Some((width, height, fitted.clone()));

        // Return Value::Image - conversion to Frame happens at the boundary (preview/render)
        Ok(vec![
            Value::Image(fitted)
        ])
    }
}

// Inventory registration for ImageSource
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(ImageSource::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::ImageFormat;

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn conforms_a_mismatched_image_to_the_graphs_current_resolution() {
        let mut source = ImageSource::new();
        source.set_image(Arc::new(U8Image {
            pixels: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
            format: ImageFormat::Rgba8,
        }));

        let values = source.execute(&context(3, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                assert_eq!(out.width, 3);
                assert_eq!(out.height, 1);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn reuses_the_same_arc_when_the_resolution_is_unchanged() {
        let mut source = ImageSource::new();
        source.set_image(Arc::new(U8Image {
            pixels: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
            format: ImageFormat::Rgba8,
        }));

        let first = source.execute(&context(3, 1), &[]).unwrap();
        let second = source.execute(&context(3, 1), &[]).unwrap();

        match (&first[0], &second[0]) {
            (Value::Image(a), Value::Image(b)) => assert!(Arc::ptr_eq(a, b)),
            other => panic!("expected two images, got {:?}", other),
        }
    }

    #[test]
    fn re_fits_when_the_resolution_changes() {
        let mut source = ImageSource::new();
        source.set_image(Arc::new(U8Image {
            pixels: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
            format: ImageFormat::Rgba8,
        }));

        source.execute(&context(3, 1), &[]).unwrap();
        let values = source.execute(&context(5, 5), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                assert_eq!(out.width, 5);
                assert_eq!(out.height, 5);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }
}
