// SPDX-License-Identifier: MPL-2.0
#include "ofe_v1.h"

#include <godot_cpp/classes/ref_counted.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/variant/dictionary.hpp>

#include <cstddef>
#include <cstdint>
#include <limits>

namespace godot {
static_assert(sizeof(ofe_state_v1) == 32);
static_assert(offsetof(ofe_state_v1, frame) == 8);
static_assert(offsetof(ofe_state_v1, h) == 28);

class OFEBattle : public RefCounted {
    GDCLASS(OFEBattle, RefCounted)

    uint64_t token = 0;

  protected:
    static void _bind_methods() {
        ClassDB::bind_method(D_METHOD("open"), &OFEBattle::open);
        ClassDB::bind_method(D_METHOD("close"), &OFEBattle::close);
        ClassDB::bind_method(D_METHOD("submit", "frame", "axis"), &OFEBattle::submit);
        ClassDB::bind_method(D_METHOD("step"), &OFEBattle::step);
        ClassDB::bind_method(D_METHOD("read_state"), &OFEBattle::read_state);
    }

  public:
    int64_t open() {
        if (token != 0) {
            return OFE_INVALID_ARGUMENT;
        }
        return ofe_v1_create(&token);
    }

    int64_t close() {
        const int32_t status = ofe_v1_destroy(token);
        if (status == OFE_OK) {
            token = 0;
        }
        return status;
    }

    int64_t submit(int64_t frame, int64_t axis) {
        if (frame < 0 || axis < std::numeric_limits<int32_t>::min() ||
            axis > std::numeric_limits<int32_t>::max()) {
            return OFE_INVALID_ARGUMENT;
        }
        return ofe_v1_submit(token, static_cast<uint64_t>(frame), static_cast<int32_t>(axis));
    }

    int64_t step() { return ofe_v1_step(token); }

    Dictionary read_state() const {
        ofe_state_v1 state{};
        state.size = sizeof(state);
        state.version = 1;
        Dictionary result;
        const int32_t status = ofe_v1_read(token, &state);
        result["status"] = status;
        if (status == OFE_OK) {
            if (state.frame > static_cast<uint64_t>(std::numeric_limits<int64_t>::max())) {
                result["status"] = static_cast<int64_t>(OFE_LIMIT);
                return result;
            }
            result["frame"] = static_cast<int64_t>(state.frame);
            result["entity_id"] = static_cast<int64_t>(state.entity_id);
            result["x"] = state.x;
            result["z"] = state.z;
            result["h"] = state.h;
        }
        return result;
    }

    ~OFEBattle() override {
        if (token != 0) {
            const int32_t status = ofe_v1_destroy(token);
            ERR_FAIL_COND_MSG(status != OFE_OK, "OFE battle cleanup failed");
        }
    }
};

static void initialize(ModuleInitializationLevel level) {
    if (level == MODULE_INITIALIZATION_LEVEL_SCENE) {
        GDREGISTER_CLASS(OFEBattle);
    }
}

static void uninitialize(ModuleInitializationLevel) {}
} // namespace godot

extern "C" {
GDExtensionBool GDE_EXPORT ofe_godot_init(GDExtensionInterfaceGetProcAddress get_proc_address,
                                          GDExtensionClassLibraryPtr library,
                                          GDExtensionInitialization *initialization) {
    godot::GDExtensionBinding::InitObject init(get_proc_address, library, initialization);
    init.register_initializer(godot::initialize);
    init.register_terminator(godot::uninitialize);
    init.set_minimum_library_initialization_level(godot::MODULE_INITIALIZATION_LEVEL_SCENE);
    return init.init();
}
}
