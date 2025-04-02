#pragma once

#ifdef __cplusplus
extern "C" {
#endif


typedef struct LCRQImpl* LCRQ;

LCRQ lcrq_create(int max_threads);
void lcrq_destroy(LCRQ queue);
int lcrq_push(LCRQ queue, void* item, const int tid);
int lcrq_pop(LCRQ queue, void** item, const int tid);


#ifdef __cplusplus
}
#endif
