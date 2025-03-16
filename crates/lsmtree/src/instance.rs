use std::sync::Arc;

use resource_manager::{LSMEvent, MemtableSwitchEvent};
use tokio::sync::mpsc;

use crate::metric::Metric;

pub struct LSMInstance {
    metric_reporter: mpsc::Sender<Metric>,
    event_emitter: mpsc::Sender<LSMEvent>,
}

impl LSMInstance {
    pub fn new(reporter: mpsc::Sender<Metric>, emitter: mpsc::Sender<LSMEvent>) -> Arc<Self> {
        Arc::new(Self {
            metric_reporter: reporter,
            event_emitter: emitter,
        })
    }

    pub async fn memtable_switch_happen(&self, event: MemtableSwitchEvent) {
    }
}