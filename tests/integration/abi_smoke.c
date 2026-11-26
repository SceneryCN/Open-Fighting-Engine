// SPDX-License-Identifier: MPL-2.0
#include "ofe_v1.h"
#include <stddef.h>
#include <stdio.h>

#define CHECK(expr)                                                                                \
    do {                                                                                           \
        if (!(expr)) {                                                                             \
            fprintf(stderr, "ABI check failed at line %d: %s\n", __LINE__, #expr);                 \
            return 1;                                                                              \
        }                                                                                          \
    } while (0)

int main(void) {
    uint64_t previous = 0;
    CHECK(sizeof(ofe_state_v1) == 32);
    CHECK(offsetof(ofe_state_v1, h) == 28);
    CHECK(ofe_v1_create(NULL) == OFE_INVALID_ARGUMENT);
    CHECK(ofe_v1_read(0, NULL) == OFE_INVALID_ARGUMENT);
    CHECK(ofe_v1_destroy(0) == OFE_INVALID_HANDLE);

    for (unsigned i = 0; i < 100000; ++i) {
        uint64_t token = 0;
        struct {
            ofe_state_v1 state;
            uint64_t sentinel;
        } extended = {{0, 1, 99, 0, 0, 0, 0}, UINT64_C(0xd3ccafebad)};
        ofe_state_v1 *state = &extended.state;
        CHECK(ofe_v1_create(&token) == OFE_OK);
        CHECK(token != 0 && token != previous);
        CHECK(ofe_v1_read(token, state) == OFE_INCOMPATIBLE);
        state->size = sizeof(extended);
        state->version = 2;
        CHECK(ofe_v1_read(token, state) == OFE_INCOMPATIBLE);
        CHECK(state->frame == 99);
        state->version = 1;
        CHECK(ofe_v1_read(token, state) == OFE_OK);
        CHECK(state->size == sizeof(extended) && state->version == 1);
        CHECK(state->frame == 0 && state->entity_id == 1);
        CHECK(state->x == 0 && state->z == 0 && state->h == 0);
        CHECK(ofe_v1_submit(token, 0, -1) == OFE_INPUT_CONFLICT);
        CHECK(ofe_v1_submit(token, 1, -2) == OFE_INVALID_ARGUMENT);
        CHECK(ofe_v1_submit(token, 1, -1) == OFE_OK);
        CHECK(ofe_v1_submit(token, 1, 1) == OFE_INPUT_CONFLICT);
        CHECK(ofe_v1_step(token) == OFE_OK);
        CHECK(ofe_v1_step(token) == OFE_OK);
        CHECK(ofe_v1_read(token, state) == OFE_OK);
        CHECK(state->frame == 2 && state->x == -1);
        CHECK(extended.sentinel == UINT64_C(0xd3ccafebad));
        CHECK(ofe_v1_destroy(token) == OFE_OK);
        CHECK(ofe_v1_submit(token, 3, 0) == OFE_INVALID_HANDLE);
        CHECK(ofe_v1_destroy(token) == OFE_INVALID_HANDLE);
        CHECK(ofe_v1_read(token, state) == OFE_INVALID_HANDLE);
        CHECK(state->frame == 2);
        CHECK(ofe_v1_step(token) == OFE_INVALID_HANDLE);
        previous = token;
    }
    puts("OFE_ABI_PASS");
    return 0;
}
