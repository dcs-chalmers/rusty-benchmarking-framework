// wrapper.cpp
#include "wrapper.hpp"
#include <boost/lockfree/queue.hpp>
#include "concurrentqueue/concurrentqueue.h"
#include "LCRQueue.hpp"
#include "cpp-ring-queues-research/include/LPRQueue.hpp"

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

struct LCRQImpl {
    LCRQueue<void> queue;
    explicit LCRQImpl(int max_threads)
        : queue(max_threads) {}
};

LCRQ lcrq_create(int max_threads) {
    return new LCRQImpl(max_threads);
}

void lcrq_destroy(LCRQ queue) {
    delete queue;
}

int lcrq_push(LCRQ queue, void* item, const int tid) {
    queue->queue.enqueue(item, tid);
    return 1;
}

int lcrq_pop(LCRQ queue, void** item, const int tid) {
    void* tmp = queue->queue.dequeue(tid);
    if (tmp != nullptr) {
        *item = tmp;
        return 1;
    } else return 0;
}

struct LPRQImpl {
    LPRQueue<void> queue;
    explicit LPRQImpl(int max_threads)
        : queue(max_threads) {}
};

LPRQ lprq_create(int max_threads) {
    return new LPRQImpl(max_threads);
}

void lprq_destroy(LPRQ queue) {
    delete queue;
}

int lprq_push(LPRQ queue, void* item, const int tid) {
    queue->queue.enqueue(item, tid);
    return 1;
}

int lprq_pop(LPRQ queue, void** item, const int tid) {
    void* tmp = queue->queue.dequeue(tid);
    if (tmp != nullptr) {
        *item = tmp;
        return 1;
    } else return 0;
}
