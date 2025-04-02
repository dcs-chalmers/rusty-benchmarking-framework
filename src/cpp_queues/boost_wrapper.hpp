// wrapper.hpp
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct BoostLockfreeQueueImpl* BoostLockfreeQueue;
BoostLockfreeQueue boost_queue_create(unsigned int capacity);
void boost_queue_destroy(BoostLockfreeQueue queue);
int boost_queue_push(BoostLockfreeQueue queue, void* item);
int boost_queue_pop(BoostLockfreeQueue queue, void** item);


#ifdef __cplusplus
}
#endif
