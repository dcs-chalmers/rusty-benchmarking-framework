use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    time::Instant,
};

#[derive(PartialEq, Clone)]
enum OperationType {
    Enqueue,
    Dequeue,
}

#[derive(PartialEq, Clone)]
enum EventType {
    Invocation,
    Return,
}

#[derive(PartialEq, Clone)]
pub struct LoggedOperation<T: PartialEq> {
    id: T,
    timestamp: Instant,
    operation: OperationType,
    kind: EventType,
}

pub struct ThreadLogger<T: PartialEq> {
    log: Vec<LoggedOperation<T>>,
    op_start: Option<Instant>,
}

impl<T: PartialEq + Clone> ThreadLogger<T> {
    pub fn new() -> Self {
        ThreadLogger {
            log: Vec::new(),
            op_start: None,
        }
    }

    fn start_operation(&mut self) {
        assert!(self.op_start.is_none());
        self.op_start = Some(Instant::now());
    }

    pub fn invoke_enqueue(&mut self, id: T) {
        self.log.push(LoggedOperation {
            id,
            timestamp: Instant::now(),
            operation: OperationType::Enqueue,
            kind: EventType::Invocation,
        });
    }

    pub fn invoke_dequeue(&mut self) {
        self.start_operation();
    }

    pub fn complete_enqueue(&mut self, id: T) {
        self.log.push(LoggedOperation {
            id,
            timestamp: Instant::now(),
            operation: OperationType::Enqueue,
            kind: EventType::Return,
        });
    }

    pub fn complete_dequeue(&mut self, id: T) {
        self.log.push(LoggedOperation {
            id: id.clone(),
            timestamp: self
                .op_start
                .expect("Must start dequeue before completing it."),
            operation: OperationType::Dequeue,
            kind: EventType::Invocation,
        });
        self.op_start = None;

        self.log.push(LoggedOperation {
            id,
            timestamp: Instant::now(),
            operation: OperationType::Dequeue,
            kind: EventType::Return,
        });
    }
}

pub fn check_linearizability<T: PartialEq + Eq + Clone + Hash>(
    histories: &[ThreadLogger<T>],
) -> bool {
    let mut events_ascending: Vec<LoggedOperation<T>> = histories
        .iter()
        .flat_map(|th| th.log.iter().cloned())
        .collect();
    events_ascending.sort_by_key(|event| event.timestamp);

    let mut id_to_enq_start = HashMap::new();
    let mut ordered_enq_ends = indexset::BTreeSet::new(); // Probably use something else
    let mut pending_enqs = HashSet::new();
    let mut null_starts = HashMap::new();

    let mut min_size: usize = 0;
    let mut size_tree = todo(); // Range minimum query tree

    for event in events_ascending {
        if event.operation == OperationType::Enqueue && event.kind == EventType::Invocation {
            id_to_enq_start[&event.id] = event.timestamp;
        } else if event.operation == OperationType::Dequeue && event.kind == EventType::Invocation {
            if event.id.is_null() {
                null_starts[&event.id] = event.timestamp;
            } else {
                ordered_enq_ends.remove(&event.id);
                if pending_enqs.contains(&event.id) {
                    pending_enqs.remove(&event.id);
                } else {
                    min_size -= 1;
                    size_tree.insert(min_size, event.timestamp);
                }
            }
        } else if event.operation == OperationType::Enqueue && event.kind == EventType::Return {
            ordered_enq_ends.insert(event.id); // Should be sorted by the order of which it was inserted
            if pending_enqs.contains(&event.id) {
                min_size += 1;
                size_tree.insert(min_size, event.timestamp);
            }
        } else if event.operation == OperationType::Dequeue && event.kind == EventType::Return {
            if event.id.is_null() {
                let start_dequeue_time = null_starts[event.id];
                // If not possible to find a time during dequeue where the queue could have been empty, it must not be linearizable
                if size_tree.find_min(start_dequeue_time, event.timestamp) > 0 {
                    return false;
                }
            } else {
                if let Some(start_enqueue_time) = id_to_enq_start.get(&event.id) {
                    if ordered_enq_ends.count_smaller_than(start_enqueue_time) > 0 {
                        // Some items must still be in the queue
                        return false;
                    }
                } else {
                    // Some dequeue finished before its enqueue started
                    return false;
                }
            }
        }
    }

    true
}
