#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct LPRQImpl* LPRQ;

LPRQ lprq_create(int max_threads);
void lprq_destroy(LPRQ queue);
int lprq_push(LPRQ queue, void* item, const int tid);
int lprq_pop(LPRQ queue, void** item, const int tid);


#ifdef __cplusplus
}
#endif
