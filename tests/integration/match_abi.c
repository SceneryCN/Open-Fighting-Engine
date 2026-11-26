// SPDX-License-Identifier: MPL-2.0
#include "ofe_v1.h"
#include "ofe_v2.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(expr)                                                                                \
    do {                                                                                           \
        if (!(expr)) {                                                                             \
            fprintf(stderr, "match ABI line %d: %s\n", __LINE__, #expr);                           \
            return 1;                                                                              \
        }                                                                                          \
    } while (0)

int main(int argc, char **argv) {
    uint64_t handle = 0, legacy = 0, fresh = 0;
    uint32_t size = 0, written = 0;
    CHECK(argc == 3);
    CHECK(ofe_v1_create(&legacy) == OFE_OK);
    CHECK(ofe_v2_create(argv[1], argv[2], &handle) == OFE_OK);
    CHECK(handle != 0 && handle != legacy);
    CHECK(ofe_v1_destroy(handle) == OFE_INVALID_HANDLE);
    CHECK(ofe_v2_destroy(legacy) == OFE_INVALID_HANDLE);
    CHECK(ofe_v2_snapshot(handle, NULL, 0, &size) == OFE_LIMIT && size > 1);
    uint8_t *before = malloc((size_t)size + 1);
    uint8_t *after = malloc((size_t)size + 1);
    CHECK(before && after);
    memset(before, 0xa5, (size_t)size + 1);
    CHECK(ofe_v2_snapshot(handle, before, size - 1, &written) == OFE_LIMIT);
    CHECK(written == size);
    for (uint32_t i = 0; i <= size; i++)
        CHECK(before[i] == 0xa5);
    CHECK(ofe_v2_snapshot(handle, before, size, &written) == OFE_OK && written == size);
    CHECK(before[size] == 0xa5);
    CHECK(ofe_v2_step(handle, 2, 0, 0) != OFE_OK);
    CHECK(ofe_v2_step(handle, 1, 0, UINT32_C(1) << 31) == OFE_INVALID_ARGUMENT);
    CHECK(ofe_v2_step(handle, 1, 0, 3) == OFE_INVALID_ARGUMENT);
    CHECK(ofe_v2_step(handle, 1, 0, (2u << 8)) == OFE_INVALID_ARGUMENT);
    CHECK(ofe_v2_step(handle, 1, (1u << 4) | (1u << 8), 0) == OFE_INVALID_ARGUMENT);
    CHECK(ofe_v2_snapshot(handle, after, size, &written) == OFE_OK && written == size);
    CHECK(memcmp(before, after, size) == 0);
    CHECK(ofe_v2_step(handle, 1, 1u << 4, 0) == OFE_OK);
    CHECK(ofe_v2_step(handle, 2, 0, 0) == OFE_OK);
    CHECK(ofe_v2_destroy(handle) == OFE_OK);
    CHECK(ofe_v2_step(handle, 3, 0, 0) == OFE_INVALID_HANDLE);
    CHECK(ofe_v2_snapshot(handle, after, size, &written) == OFE_INVALID_HANDLE);
    CHECK(ofe_v2_create(argv[1], argv[2], &fresh) == OFE_OK && fresh != handle);
    CHECK(ofe_v2_snapshot(fresh, after, size, &written) == OFE_OK && written == size);
    CHECK(memcmp(before, after, size) == 0);
    CHECK(ofe_v2_destroy(fresh) == OFE_OK);
    CHECK(ofe_v1_destroy(legacy) == OFE_OK);
    free(before);
    free(after);
    puts("OFE_MATCH_ABI_PASS");
    return 0;
}
