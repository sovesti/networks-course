use std::sync::{Condvar, Mutex, MutexGuard};

pub struct Pool {
    threads: Mutex<u16>,
    concurrency: u16,
    ceil: Condvar,
}

impl Pool {
    pub fn new(concurrency: u16) -> Self {
        Self {
            threads: Mutex::new(0),
            concurrency,
            ceil: Condvar::new(),
        }
    }

    pub fn take(&self) {
        *self
            .ceil
            .wait_while(self.lock(), |threads| *threads >= self.concurrency)
            .unwrap() += 1;
    }

    pub fn release(&self) {
        let mut locked = self.lock();
        *locked -= 1;
        self.ceil.notify_one();
    }

    fn lock(&self) -> MutexGuard<'_, u16> {
        self.threads.lock().unwrap()
    }
}
