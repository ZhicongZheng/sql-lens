//! Capture pipeline primitives for SQL Lens.

mod classifier;
mod pipeline;

pub use classifier::{DEFAULT_SLOW_THRESHOLD_MS, SlowQueryClassifier};
pub use pipeline::{
    CaptureEventPublisher, CaptureEventReceiver, CaptureOverloadPolicy, CapturePipeline,
    CapturePipelineConfig, CapturePipelineStats, CapturePublishError, CapturePublishOutcome,
};
