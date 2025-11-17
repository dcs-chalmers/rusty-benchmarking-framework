#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MoodyCamelConcurrentQueueImpl* MoodyCamelConcurrentQueue;

MoodyCamelConcurrentQueue moody_camel_create();

void moody_camel_destroy(MoodyCamelConcurrentQueue queue);

int moody_camel_push(MoodyCamelConcurrentQueue queue, void* item);

int moody_camel_pop(MoodyCamelConcurrentQueue queue, void** item);

#ifdef __cplusplus
}
#endif
