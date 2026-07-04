use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

#[derive(Clone)]
pub struct Pool {
    semaphore: Arc<Semaphore>,
    set: Arc<Mutex<JoinSet<()>>>,
}

impl Pool {
    pub fn new(max_workers: usize) -> Self {
        Pool {
            semaphore: Arc::new(Semaphore::new(max_workers)),
            set: Arc::new(Mutex::new(JoinSet::new())),
        }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let sem = self.semaphore.clone();
        self.set.lock().unwrap().spawn(async move {
            let _permit = sem
                .acquire_owned()
                .await
                .expect("semaphore has been closed");
            f.await
        });
    }

    pub async fn join(&self) {
        let mut set = std::mem::take(&mut *self.set.lock().unwrap());
        while let Some(result) = set.join_next().await {
            if let Err(e) = result {
                log::error!("pool task failed: {}", e);
            }
        }
    }
}
