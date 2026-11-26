/* SPDX-License-Identifier: MPL-2.0 */
#ifndef OFE_V2_H
#define OFE_V2_H
#include "ofe_v1.h"

#ifdef __cplusplus
extern "C" {
#endif

/* P4 desktop match. Directory paths are NUL-terminated UTF-8 strings.
 * Buffers are caller-owned, suitably aligned, exclusive and non-overlapping.
 * A v2 token cannot be used by v1. The caller orders operations on a match. */
int32_t ofe_v2_create(const char *p1_dir, const char *p2_dir, uint64_t *out);
/* P5 mode: 0 duel (2), 1 team (4), 2 free-for-all (4), 3 training (2).
 * Caller retains all arrays; IDs follow array order starting at one. */
int32_t ofe_v2_create_roster(const char *const *dirs, uint32_t count, uint32_t mode, uint64_t *out);
int32_t ofe_v2_step_roster(uint64_t handle, uint64_t frame, const uint32_t *masks, uint32_t count);
/* Enabled is 0 or 1. Valid only on training handles. */
int32_t ofe_v2_set_infinite_health(uint64_t handle, uint32_t enabled);
int32_t ofe_v2_destroy(uint64_t handle);
/* Bits 0..6: left/right/jump/lane/attack/guard/dash. Bits 8..23 hold an
 * explicit zero-based move index PLUS ONE. Other bits must be zero. */
int32_t ofe_v2_step(uint64_t handle, uint64_t frame, uint32_t p1, uint32_t p2);
/* JSON (no terminating NUL), or encoded PNG. Supply NULL,0 to query length.
 * OFE_LIMIT reports required length without writing the byte buffer.
 * Successful calls set length to the number of bytes written. */
int32_t ofe_v2_snapshot(uint64_t handle, uint8_t *out, uint32_t capacity, uint32_t *length);
int32_t ofe_v2_image(uint64_t handle, uint32_t image_id, uint8_t *out, uint32_t capacity,
                     uint32_t *length);
/* Training recordings start at frame zero. Reset retains the previous tape.
 * play_load validates a portable recording against this match's combat rules.
 * On divergence play_step leaves the last verified frame committed.
 * replay_export and replay_info follow the snapshot buffer contract above. */
int32_t ofe_v2_training_reset(uint64_t handle);
int32_t ofe_v2_record_begin(uint64_t handle);
int32_t ofe_v2_record_end(uint64_t handle);
int32_t ofe_v2_play_begin(uint64_t handle);
int32_t ofe_v2_play_step(uint64_t handle);
int32_t ofe_v2_play_load(uint64_t handle, const uint8_t *data, uint32_t length);
int32_t ofe_v2_replay_export(uint64_t handle, uint8_t *out, uint32_t capacity, uint32_t *length);
int32_t ofe_v2_replay_info(uint64_t handle, uint8_t *out, uint32_t capacity, uint32_t *length);
#ifdef __cplusplus
}
#endif
#endif
