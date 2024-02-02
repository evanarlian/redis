use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

struct Worker {
    id: usize,
    handle: thread::JoinHandle<()>,
}
impl Worker {
    fn new<F>(id: usize, rx: Arc<Mutex<mpsc::Receiver<F>>>) -> Worker
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
                Err(_e) => {
                    eprintln!("pipe closed, no more jobs, worker {} shutting down", id);
                    break;
                }
            };
        });
        Worker { id, handle }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    tx: mpsc::Sender<Job>,
    workers: Vec<Worker>,
}
impl ThreadPool {
    pub fn build(num_workers: usize) -> ThreadPool {
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = vec![];
        for i in 0..num_workers {
            workers.push(Worker::new(i, Arc::clone(&rx)));
        }
        ThreadPool { tx, workers }
    }

    pub fn submit<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx.send(Box::new(job)).unwrap();
    }
}
