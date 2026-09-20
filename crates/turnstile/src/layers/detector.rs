use std::sync::Arc;

use pinnacle_core::Layer;

use crate::services::{Detect, Detector};

#[derive(Clone)]
pub struct DetectorLayer {
    detector: Arc<dyn Detector>,
}

impl DetectorLayer {
    pub fn new(detector: Arc<dyn Detector>) -> Self {
        Self { detector }
    }
}

impl<S> Layer<S> for DetectorLayer {
    type Service = Detect<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Detect {
            detector: self.detector.clone(),
            inner,
        }
    }
}
