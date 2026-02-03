/// Trait that all queues need to implement
pub trait ConcurrentQueue<T> {
    /// Returns a handle that exposes the queue API
    fn register(&self) -> impl HandleQueue<T>;
    /// Returns the name of the queue.
    fn get_id(&self) -> String;
    /// Used to create a new queue.
    /// `size` is discarded for unbounded queues.
    fn new(size: usize) -> Self;
}

/// Trait that exposes the correct API for queues
pub trait HandleQueue<T> {
    /// Pushes an item to the queue.
    /// If it fails, returns the item pushed.
    fn push(&mut self, item: T) -> Result<(), T>;
    /// Pops an item from the queue.
    fn pop(&mut self) -> Option<T>;
}
