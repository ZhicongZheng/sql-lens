//! Capture pipeline primitives for SQL Lens.

mod pipeline;

pub use pipeline::{
    CaptureEventPublisher, CaptureEventReceiver, CaptureOverloadPolicy, CapturePipeline,
    CapturePipelineConfig, CapturePipelineStats, CapturePublishError, CapturePublishOutcome,
};
