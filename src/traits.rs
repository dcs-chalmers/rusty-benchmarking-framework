/// One of the traits that all queues implemented in the benchmark
/// needs to implement.
pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
    /// Returns the name of the queue.
    fn get_id(&self) -> String;
    /// Used to create a new queue.
    /// `size` is discarded for unbounded queues.
    fn new(size: usize) -> Self;
}

/// One of the traits all queues implemented in the benchmark
/// needs to implement.
pub trait Handle<T> {
    /// Pushes an item to the queue.
    /// If it fails, returns the item pushed.
    fn push(&mut self, item: T) -> Result<(), T>;
    /// Pops an item from the queue.
    fn pop(&mut self) -> Option<T>;
}
