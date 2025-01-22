use lockfree;

pub fn start_benchmark() {
}

pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
}

pub trait Handle<T> {
    fn push(&self, item: T);
    fn pop(&self) -> Option<T>;
}

pub struct LFQueueHandle<'a, T> {
    queue: &'a LFQueue<T>
}

pub struct LFQueue<T> {
    lfq: lockfree::queue::Queue<T>
}

impl<T> ConcurrentQueue<T> for LFQueue<T> {
    
    fn register(&self) -> impl Handle<T> {
        LFQueueHandle {
            queue: self,
        }
    }
}

impl<T> Handle<T> for LFQueueHandle<'_, T> {
    fn push(&self, item: T) {
        self.queue.lfq.push(item);
    }
    
    fn pop(&self) -> Option<T> {
        self.queue.lfq.pop()
    }
}