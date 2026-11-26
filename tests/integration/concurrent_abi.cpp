// SPDX-License-Identifier: MPL-2.0
// Independent black-box test of the public C API. Build with assertions enabled.
#include "ofe_v1.h"

#include <array>
#include <atomic>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <thread>

#ifdef NDEBUG
#error This test needs assertions enabled
#endif

int main() {
    uint64_t handle = 0;
    assert(ofe_v1_create(&handle) == OFE_OK);
    std::atomic<int> accepted{0};
    std::array<std::thread, 8> workers;
    for (auto &worker : workers) {
        worker = std::thread([&] {
            const int status = ofe_v1_submit(handle, 1, -1);
            assert(status == OFE_OK || status == OFE_INPUT_CONFLICT);
            if (status == OFE_OK) {
                ++accepted;
            }
            for (int i = 0; i < 512; ++i) {
                uint64_t isolated = 0;
                assert(ofe_v1_create(&isolated) == OFE_OK);
                assert(isolated != handle && isolated != 0);
                assert(ofe_v1_step(isolated) == OFE_OK);
                assert(ofe_v1_destroy(isolated) == OFE_OK);
                assert(ofe_v1_step(isolated) == OFE_INVALID_HANDLE);
            }
        });
    }
    for (auto &worker : workers) {
        worker.join();
    }
    assert(accepted == 1);
    assert(ofe_v1_step(handle) == OFE_OK);
    struct Output {
        ofe_state_v1 state;
        uint64_t canary;
    } output{{sizeof(Output), 1, 0, 0, 0, 0, 0}, UINT64_MAX};
    assert(ofe_v1_read(handle, &output.state) == OFE_OK);
    assert(output.state.frame == 1 && output.state.x == -1);
    assert(output.state.size == sizeof(Output) && output.canary == UINT64_MAX);
    assert(ofe_v1_destroy(handle) == OFE_OK);
    assert(ofe_v1_read(handle, &output.state) == OFE_INVALID_HANDLE);
    assert(output.state.frame == 1 && output.canary == UINT64_MAX);
    std::puts("PASS: 8 submitters, one accepted, 4096 isolated lifecycles, ABI tail preserved");
}
