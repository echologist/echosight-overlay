use std::{
    collections::HashMap,
    io,
    path::{Component, Path, PathBuf},
    sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, Weak},
};

pub(super) fn unpoison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Lexical normalization, like path.resolve: does not probe user directories or
/// follow symlinks. Relative paths are frozen at the call's current directory.
pub(super) fn resolve(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut result = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}

#[derive(Default)]
struct Queue {
    turns: Mutex<(u64, u64)>, // next ticket, serving ticket
    changed: Condvar,
}
static QUEUES: LazyLock<Mutex<HashMap<PathBuf, Weak<Queue>>>> = LazyLock::new(Mutex::default);

pub(super) struct Ticket {
    queue: Arc<Queue>,
    number: u64,
}
impl Ticket {
    pub(super) fn new(path: &Path) -> Self {
        let mut registry = unpoison(&QUEUES);
        registry.retain(|_, queue| queue.strong_count() != 0);
        let queue = registry
            .get(path)
            .and_then(Weak::upgrade)
            .unwrap_or_else(|| {
                let queue = Arc::new(Queue::default());
                registry.insert(path.to_path_buf(), Arc::downgrade(&queue));
                queue
            });
        let number = {
            let mut turns = unpoison(&queue.turns);
            let number = turns.0;
            turns.0 += 1;
            number
        };
        Self { queue, number }
    }
    pub(super) fn wait(&self) {
        let mut turns = unpoison(&self.queue.turns);
        while turns.1 != self.number {
            turns = self
                .queue
                .changed
                .wait(turns)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        // Even an unwinding hook cannot leave a hole in the FIFO queue.
        self.wait();
        unpoison(&self.queue.turns).1 += 1;
        self.queue.changed.notify_all();
    }
}
