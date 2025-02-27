// wrapper.hpp
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// Opaque pointer to hide C++ implementation details
typedef struct BoostLockfreeQueueImpl* BoostLockfreeQueue;

// Create a new queue with the specified capacity
BoostLockfreeQueue boost_queue_create(unsigned int capacity);

// Destroy a queue
void boost_queue_destroy(BoostLockfreeQueue queue);

// Push an item to the queue, returns 1 on success, 0 on failure
int boost_queue_push(BoostLockfreeQueue queue, void* item);

// Pop an item from the queue, returns 1 on success, 0 on failure
int boost_queue_pop(BoostLockfreeQueue queue, void** item);

#ifdef __cplusplus
}
#endif
