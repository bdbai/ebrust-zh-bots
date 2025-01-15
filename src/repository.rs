use flume::Sender;
use futures::channel::oneshot;
use rusqlite::Connection;
use thiserror::Error;

mod id;
pub mod playground_record;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("worker has stopped")]
    WorkerGone,
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

type ConnExecutor = Box<dyn FnOnce(&mut Connection) + Send>;

#[derive(Clone)]
pub struct Repository {
    db_tx: Sender<ConnExecutor>,
}

refinery::embed_migrations!("migrations");

pub fn init_db(mut conn: Connection) -> RepositoryResult<Repository> {
    migrations::runner().run(&mut conn).expect("db migration");

    let (tx, rx) = flume::bounded(10);
    let res = Repository { db_tx: tx };
    std::thread::spawn(move || {
        while let Ok(work) = rx.recv() {
            // TODO: catch unwind
            work(&mut conn);
        }
    });
    Ok(res)
}

impl Repository {
    async fn with_db<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> RepositoryResult<T> + Send + 'static,
    ) -> RepositoryResult<T> {
        let (result_tx, result_rx) = oneshot::channel();
        self.db_tx
            .send(Box::new(|conn| {
                let result = work(conn);
                let _ = result_tx.send(result);
            }))
            .map_err(|_| RepositoryError::WorkerGone)?;
        result_rx.await.map_err(|_| RepositoryError::WorkerGone)?
    }
}
