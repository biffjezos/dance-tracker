use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    Operation,
    OperationDescriptor,
    OperationError,
    Input,
    Value,
    metadata::{
        OperationCategory,
        OperationMetadata,
        OutputKind,
    },
};

use crate::graphics::Video;

pub struct VideoSource {
    pub video: Option<Arc<Video>>,
}

impl VideoSource {
    pub fn new() -> Self {
        Self {
            video: None,
        }
    }

    pub fn set_video(&mut self, video: Arc<Video>) {
        self.video = Some(video);
    }

    pub fn get_video(&self) -> Option<Arc<Video>> {
        self.video.clone()
    }
}

impl Operation for VideoSource {

    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "video_source",
            menu: "INPUT",
            label: "LOAD VIDEO",
            action: None,
            ui_action: Some("open_video_picker"),
            create_node: None,
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
            display_name: "Video Source",
            category: OperationCategory::Source,
            input_count: 0,
            outputs: vec![OutputKind::Video],
        }
    }

    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {

        let video = self
            .video
            .clone()
            .ok_or_else(|| {
                OperationError::SourceNotFound(
                    "Video not loaded".to_string()
                )
            })?;

        Ok(vec![
            Value::Video(video)
        ])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(VideoSource::new())
    }
}