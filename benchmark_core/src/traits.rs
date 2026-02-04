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

/// Trait that all priority queues need to implement
pub trait ConcurrentPriorityQueue<P: Ord, T> {
    /// Returns a handle that exposes the priority queue API
    fn register(&self) -> impl HandlePriorityQueue<P, T>;
    /// Returns the name of the queue.
    fn get_id(&self) -> String;
    /// Used to create a new queue.
    /// `size` is discarded for unbounded queues.
    fn new(size: usize) -> Self;
}

/// Trait that exposes the correct API for priority queues
pub trait HandlePriorityQueue<P: Ord, T> {
    /// Inserts an item into the priority queue.
    /// In case of failure, returns the item and priority that failed to insert.
    fn insert(&mut self, priority: P, item: T) -> Result<(), (P, T)>;
    /// Deletes the minimum item from the queue
    /// Returns nothing if the queue is empty
    fn delete_min(&mut self) -> Option<T>;
    /// Checks if the priority queue is empty
    fn is_empty(&mut self) -> bool;
    /// Peeks at the smallest key-value pair but doesn't remove it from the
    /// queue
    /// Returns an enum wrapped in a Some() of the form (key, value) if there 
    /// exists a smallest value
    /// Returns None if there is no item in the queue
    fn min(&mut self) -> Option<(&P, &T)>;
}
