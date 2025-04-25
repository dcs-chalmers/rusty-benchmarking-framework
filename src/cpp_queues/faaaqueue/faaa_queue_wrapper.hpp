#pragma once

#ifdef __cplusplus
extern "C" {
#endif


typedef struct FAAAQImpl* FAAAQ;

FAAAQ faaaq_create(int max_threads);
void faaaq_destroy(FAAAQ queue);
int faaaq_push(FAAAQ queue, void* item, const int tid);
int faaaq_pop(FAAAQ queue, void** item, const int tid);


#ifdef __cplusplus
}
#endif
