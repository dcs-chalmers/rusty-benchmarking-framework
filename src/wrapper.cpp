// wrapper.cpp
#include "wrapper.hpp"
#include <boost/lockfree/queue.hpp>
#include "concurrentqueue/concurrentqueue.h"

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

struct MoodyCamelConcurrentQueueImpl {
    moodycamel::ConcurrentQueue<void*> queue;

    explicit MoodyCamelConcurrentQueueImpl()
        : queue() {}
};

MoodyCamelConcurrentQueue moody_camel_create() {
    return new MoodyCamelConcurrentQueueImpl();
}

void moody_camel_destroy(MoodyCamelConcurrentQueue queue) {
    delete queue;
}

int moody_camel_push(MoodyCamelConcurrentQueue queue, void *item) {
    return queue->queue.enqueue(item) ? 1 : 0;
}

int moody_camel_pop(MoodyCamelConcurrentQueue queue, void **item) {
    return queue->queue.try_dequeue(*item) ? 1 : 0;
}
