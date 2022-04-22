use std::future::Future;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct GracefulShutdown(Arc<RwLock<bool>>);

impl GracefulShutdown {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(false)))
    }

    pub async fn start_listener(self) {
        self.listen(Self::stop_gracefully).await;
    }

    pub async fn get(&self) -> bool {
        *self.0.read().await
    }

    async fn listen<F, Fut>(self, handler: F)
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigquit = signal(SignalKind::quit()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {
                handler(self).await;
            },
            _ = sigquit.recv() => {
                handler(self).await;
            },
            _ = sigterm.recv() => {
                handler(self).await;
            },
        }
    }

    async fn stop_forcibly(self) {
        println!("Force stopping...");
    }

    async fn stop_gracefully(self) {
        println!("Gracefully stopping... (press Ctrl+C again to force)");
        *self.0.write().await = true;

        self.listen(Self::stop_forcibly).await;
    }
}
