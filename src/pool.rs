use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

struct Worker {
    id: usize,
    handle: JoinHandle<()>,
}
impl Worker {
    fn new<F>(id: usize, rx: Arc<Mutex<Receiver<F>>>) -> Worker
    where
        F: FnOnce() + Send + 'static,
    {
        eprintln!("worker {} is starting", id);
        let handle = thread::spawn(move || loop {
            let new_job = rx.lock().unwrap().recv();
            match new_job {
                Ok(f) => {
                    eprintln!("worker {} gets the job", id);
                    f();
                }
                Err(e) => break,
            };
        });
        Worker { id, handle }
    }
}

pub struct ThreadPool<F> {
    tx: Sender<F>,
    workers: Vec<Worker>,
}
impl<F> ThreadPool<F>
where
    F: FnOnce() + Send + 'static,
{
    pub fn build(num_workers: usize) -> ThreadPool<F> {
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = vec![];
        for i in 0..num_workers {
            workers.push(Worker::new(i, Arc::clone(&rx)));
        }
        ThreadPool { tx, workers }
    }

    pub fn submit(&self, job: F) {
        self.tx.send(job).unwrap();
    }
}
