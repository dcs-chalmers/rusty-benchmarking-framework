#include "moodycamel_wrapper.hpp"
#include "concurrentqueue/concurrentqueue.h"

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
