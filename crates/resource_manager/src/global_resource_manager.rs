use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::{ControlHappen, StorageEventHappen};

pub struct GlobalResourceManager<E: StorageEventHappen, C: ControlHappen> {
    event_rx: mpsc::Receiver<E>,
    control_tx: mpsc::Sender<C>,
}

impl<E: StorageEventHappen, C: ControlHappen> GlobalResourceManager<E, C> {
    pub fn new() -> Self {
        todo!()
    }

    async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await;
                },
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    self.collect_metrics().await;
                    self.make_decision().await;
                }
            }
        }
    }

    async fn handle_event(&self, event: E) {
        todo!()
    }

    async fn collect_metrics(&self) {
        todo!()
    }

    async fn make_decision(&self) {
        todo!()
    }
}