#include "lcrq_wrapper.hpp"
#include <string>
#include "LCRQueue.hpp"

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
