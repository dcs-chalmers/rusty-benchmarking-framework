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

typedef struct MoodyCamelConcurrentQueueImpl* MoodyCamelConcurrentQueue;

MoodyCamelConcurrentQueue moody_camel_create();

void moody_camel_destroy(MoodyCamelConcurrentQueue queue);

int moody_camel_push(MoodyCamelConcurrentQueue queue, void* item);

int moody_camel_pop(MoodyCamelConcurrentQueue queue, void** item);

typedef struct LCRQImpl* LCRQ;

LCRQ lcrq_create();

void lcrq_destroy(LCRQ queue);

int lcrq_push(LCRQ queue, void* item, const int tid);

int lcrq_pop(LCRQ queue, void** item, const int ti);


#ifdef __cplusplus
}
#endif
