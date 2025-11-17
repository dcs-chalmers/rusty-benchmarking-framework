#include "faaa_queue_wrapper.hpp"
#include "faaa_queue/FAAArrayQueue.hpp"

struct FAAAQImpl {
    FAAArrayQueue<void> queue;
    explicit FAAAQImpl(int max_threads)
        : queue(max_threads) {}
};

FAAAQ faaaq_create(int max_threads) {
    return new FAAAQImpl(max_threads);
}

void faaaq_destroy(FAAAQ queue) {
    delete queue;
}

int faaaq_push(FAAAQ queue, void* item, const int tid) {
    queue->queue.enqueue(item, tid);
    return 1;
}

int faaaq_pop(FAAAQ queue, void** item, const int tid) {
    void* tmp = queue->queue.dequeue(tid);
    if (tmp != nullptr) {
        *item = tmp;
        return 1;
    } else return 0;
}
