use std::sync::atomic;
use std::thread;

use crate::Move;

/*====================================================================================================================*/

fn search_thread() {
    todo!()
}

/*====================================================================================================================*/

pub struct Search {
    threads: Vec<thread::JoinHandle<()>>,

    search_active: atomic::AtomicBool,
}

impl Search {
    pub fn start_threads(&mut self, thread_count: u64) {
        assert!(thread_count > 0, "Can't start zero threads.");
        assert!(
            self.threads.is_empty(),
            "Trying to start thread with threads already running"
        );

        self.search_active.store(true, atomic::Ordering::Release);

        for _ in 0..thread_count {
            self.threads.push(thread::spawn(|| {
                search_thread();
            }));
        }
    }

    pub fn stop_threads(&mut self) {
        self.search_active.store(false, atomic::Ordering::Release);

        while !self.threads.is_empty() {
            let last = self.threads.pop().unwrap();

            last.join().expect("Failed to join on threads");
        }
    }

    pub fn inform_move(&self, move_: Move) {
        todo!()
    }

    pub fn current_best_move(&self) -> Move {
        todo!()
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.stop_threads();
    }
}
