#include "lprq_wrapper.hpp"
#include <vector>
#include "cpp-ring-queues-research/include/LPRQueue.hpp"

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
