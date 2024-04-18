use std::sync::{Condvar, Mutex};

pub struct ThreadGarageBarrier {
    lock: Mutex<CurState>,
    cv: Condvar,
}

struct CurState {
    cnt: usize,
    iter: usize,
    num_threads: usize
}

impl ThreadGarageBarrier {
  
    #[must_use]
    pub fn new(num_threads: usize) -> Self {
        Self {
            lock: Mutex::new(CurState { cnt: 0, iter: 0, num_threads }),
            cv: Condvar::new(),
        }
    }

    pub fn arrive_and_drop(&self) {
        let mut lock = self.lock.lock().unwrap();

        lock.cnt += 1;
        lock.num_threads -= 1;

        if lock.cnt >= lock.num_threads {
            lock.cnt = 0;
            lock.iter = lock.iter.wrapping_add(1);
            self.cv.notify_all();
        }
    }
    
    pub fn arrive_and_wait(&self) {
        let mut lock = self.lock.lock().unwrap();

        let old_iter = lock.iter;
        
        lock.cnt += 1;

        if lock.cnt < lock.num_threads {
            let _guard =
                self.cv.wait_while(lock, |state| state.iter == old_iter).unwrap();
        }
        else {
            lock.cnt = 0;
            lock.iter = lock.iter.wrapping_add(1);
            self.cv.notify_all();
        }
    }
}

