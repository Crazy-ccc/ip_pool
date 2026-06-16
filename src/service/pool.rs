use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct Pool {
    semaphore: Arc<Semaphore>,
}

impl Pool {
    pub fn new(max_workers: usize) -> Self {
        Pool {
            semaphore: Arc::new(Semaphore::new(max_workers)),
        }
    }

    pub fn spawn<F>(&self, f: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let sem = self.semaphore.clone();
        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            f.await
        })
    }
}
