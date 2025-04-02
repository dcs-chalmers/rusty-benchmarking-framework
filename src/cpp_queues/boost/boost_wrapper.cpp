#include "boost_wrapper.hpp"
#include <boost/lockfree/queue.hpp>

// The actual implementation
struct BoostLockfreeQueueImpl {
    boost::lockfree::queue<void*> queue;
    
    explicit BoostLockfreeQueueImpl(unsigned int capacity) 
        : queue(capacity) {}
};

BoostLockfreeQueue boost_queue_create(unsigned int capacity) {
    return new BoostLockfreeQueueImpl(capacity);
}

void boost_queue_destroy(BoostLockfreeQueue queue) {
    delete queue;
}

int boost_queue_push(BoostLockfreeQueue queue, void* item) {
    return queue->queue.push(item) ? 1 : 0;
}

int boost_queue_pop(BoostLockfreeQueue queue, void** item) {
    return queue->queue.pop(*item) ? 1 : 0;
}
