/* SPDX-License-Identifier: MPL-2.0 */
#ifndef OFE_V1_H
#define OFE_V1_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Return values apply to every entry point. Failed calls preserve output. */
enum ofe_status_v1 {
    OFE_OK = 0,
    OFE_INVALID_ARGUMENT = 1,
    OFE_INVALID_HANDLE = 2,
    OFE_INCOMPATIBLE = 3,
    OFE_INPUT_CONFLICT = 4,
    OFE_LIMIT = 5,
    OFE_INTERNAL = 6
};

typedef struct ofe_state_v1 {
    uint32_t size;    /* Set to the actual buffer size, at least 32. */
    uint32_t version; /* Set to 1. */
    uint64_t frame;
    uint32_t entity_id;
    int32_t x;
    int32_t z;
    int32_t h;
} ofe_state_v1;

/* All output storage belongs to the caller, aligned, writable and exclusive
 * during each call. No caller pointer is retained. Functions are thread safe.
 * The caller orders operations on the same battle. Token 0 is always invalid.
 */
/* Allocate a Rust battle and return its opaque token. Destroy exactly once. */
int32_t ofe_v1_create(uint64_t *out);
/* Release the battle and invalidate all copies of its token. */
int32_t ofe_v1_destroy(uint64_t handle);
/* Queue axis (-1, 0, +1) for the next numbered tick. One submission per tick. */
int32_t ofe_v1_submit(uint64_t handle, uint64_t frame, int32_t axis);
/* Simulate one 60 Hz tick using submitted input, or neutral if absent. */
int32_t ofe_v1_step(uint64_t handle);
/* Copy the 32-byte v1 prefix. Preserve caller extension bytes and size. */
int32_t ofe_v1_read(uint64_t handle, ofe_state_v1 *out);

#ifdef __cplusplus
}
#endif
#endif
