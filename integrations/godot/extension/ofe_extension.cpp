// SPDX-License-Identifier: MPL-2.0
#include "ofe_v1.h"
#include "ofe_v2.h"

#include <godot_cpp/classes/json.hpp>
#include <godot_cpp/classes/ref_counted.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/godot.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/packed_byte_array.hpp>

#include <godot_cpp/variant/packed_int32_array.hpp>
#include <godot_cpp/variant/packed_string_array.hpp>

#include <array>
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

class OFEMatch : public RefCounted {
    GDCLASS(OFEMatch, RefCounted)

    uint64_t token = 0;
    PackedByteArray snapshot_buffer;

    static Dictionary status_only(int32_t status) {
        Dictionary result;
        result["status"] = status;
        return result;
    }

  protected:
    static void _bind_methods() {
        ClassDB::bind_method(D_METHOD("open", "p1_dir", "p2_dir"), &OFEMatch::open);
        ClassDB::bind_method(D_METHOD("close"), &OFEMatch::close);
        ClassDB::bind_method(D_METHOD("step", "frame", "p1", "p2"), &OFEMatch::step);
        ClassDB::bind_method(D_METHOD("open_roster", "dirs", "mode"), &OFEMatch::open_roster);
        ClassDB::bind_method(D_METHOD("step_roster", "frame", "masks"), &OFEMatch::step_roster);
        ClassDB::bind_method(D_METHOD("set_infinite_health", "enabled"),
                             &OFEMatch::set_infinite_health);
        ClassDB::bind_method(D_METHOD("training_reset"), &OFEMatch::training_reset);
        ClassDB::bind_method(D_METHOD("record_begin"), &OFEMatch::record_begin);
        ClassDB::bind_method(D_METHOD("record_end"), &OFEMatch::record_end);
        ClassDB::bind_method(D_METHOD("play_begin"), &OFEMatch::play_begin);
        ClassDB::bind_method(D_METHOD("play_step"), &OFEMatch::play_step);
        ClassDB::bind_method(D_METHOD("replay_info"), &OFEMatch::replay_info);
        ClassDB::bind_method(D_METHOD("replay_export"), &OFEMatch::replay_export);
        ClassDB::bind_method(D_METHOD("play_load", "bytes"), &OFEMatch::play_load);
        ClassDB::bind_method(D_METHOD("snapshot"), &OFEMatch::snapshot);
        ClassDB::bind_method(D_METHOD("image", "image_id"), &OFEMatch::image);
    }

  public:
    int64_t open(const String &p1_dir, const String &p2_dir) {
        if (token != 0) {
            return OFE_INVALID_ARGUMENT;
        }
        const CharString p1 = p1_dir.utf8();
        const CharString p2 = p2_dir.utf8();
        return ofe_v2_create(p1.get_data(), p2.get_data(), &token);
    }

    int64_t open_roster(const PackedStringArray &dirs, const String &mode) {
        if (token != 0 || (dirs.size() != 2 && dirs.size() != 4)) {
            return OFE_INVALID_ARGUMENT;
        }
        const std::array<String, 4> names = {"duel", "team", "ffa", "training"};
        uint32_t which = 0;
        while (which < names.size() && names[which] != mode) {
            ++which;
        }
        if (which == names.size()) {
            return OFE_INVALID_ARGUMENT;
        }
        std::array<CharString, 4> utf8;
        std::array<const char *, 4> paths;
        for (int64_t i = 0; i < dirs.size(); ++i) {
            utf8[i] = dirs[i].utf8();
            paths[i] = utf8[i].get_data();
        }
        return ofe_v2_create_roster(paths.data(), static_cast<uint32_t>(dirs.size()), which,
                                    &token);
    }

    int64_t step_roster(int64_t frame, const PackedInt32Array &masks) {
        if (frame <= 0 || (masks.size() != 2 && masks.size() != 4)) {
            return OFE_INVALID_ARGUMENT;
        }
        std::array<uint32_t, 4> inputs;
        for (int64_t i = 0; i < masks.size(); ++i) {
            inputs[i] = static_cast<uint32_t>(masks[i]);
        }
        return ofe_v2_step_roster(token, static_cast<uint64_t>(frame), inputs.data(),
                                  static_cast<uint32_t>(masks.size()));
    }

    int64_t set_infinite_health(bool enabled) {
        return ofe_v2_set_infinite_health(token, enabled ? 1 : 0);
    }

    int64_t close() {
        const int32_t status = ofe_v2_destroy(token);
        if (status == OFE_OK) {
            token = 0;
            snapshot_buffer.clear();
        }
        return status;
    }

    int64_t step(int64_t frame, int64_t p1, int64_t p2) {
        constexpr int64_t max_input = std::numeric_limits<uint32_t>::max();
        if (frame <= 0 || p1 < 0 || p1 > max_input || p2 < 0 || p2 > max_input) {
            return OFE_INVALID_ARGUMENT;
        }
        return ofe_v2_step(token, static_cast<uint64_t>(frame), static_cast<uint32_t>(p1),
                           static_cast<uint32_t>(p2));
    }

    int64_t training_reset() { return ofe_v2_training_reset(token); }
    int64_t record_begin() { return ofe_v2_record_begin(token); }
    int64_t record_end() { return ofe_v2_record_end(token); }
    int64_t play_begin() { return ofe_v2_play_begin(token); }
    int64_t play_step() { return ofe_v2_play_step(token); }
    int64_t play_load(const PackedByteArray &data) {
        if (data.size() > std::numeric_limits<uint32_t>::max()) {
            return OFE_LIMIT;
        }
        return ofe_v2_play_load(token, data.ptr(), static_cast<uint32_t>(data.size()));
    }
    PackedByteArray replay_export() {
        PackedByteArray bytes;
        uint32_t length = 0;
        if (ofe_v2_replay_export(token, nullptr, 0, &length) != OFE_LIMIT || length == 0 ||
            bytes.resize(length) != 0 ||
            ofe_v2_replay_export(token, bytes.ptrw(), length, &length) != OFE_OK) {
            return {};
        }
        return bytes;
    }
    Dictionary snapshot() { return read_dictionary(ofe_v2_snapshot); }
    Dictionary replay_info() { return read_dictionary(ofe_v2_replay_info); }

  private:
    Dictionary read_dictionary(int32_t (*read)(uint64_t, uint8_t *, uint32_t, uint32_t *)) {
        uint32_t length = 0;
        auto capacity = static_cast<uint32_t>(snapshot_buffer.size());
        uint8_t *data = capacity == 0 ? nullptr : snapshot_buffer.ptrw();
        int32_t status = read(token, data, capacity, &length);
        if (status == OFE_LIMIT) {
            if (snapshot_buffer.resize(length) != 0) {
                return status_only(OFE_LIMIT);
            }
            status = read(token, snapshot_buffer.ptrw(), length, &length);
        }
        if (status != OFE_OK) {
            return status_only(status);
        }
        const String text =
            String::utf8(reinterpret_cast<const char *>(snapshot_buffer.ptr()), length);
        const Variant decoded = JSON::parse_string(text);
        if (decoded.get_type() != Variant::DICTIONARY) {
            return status_only(OFE_INTERNAL);
        }
        Dictionary result = decoded;
        result["status"] = static_cast<int64_t>(OFE_OK);
        return result;
    }

  public:
    PackedByteArray image(int64_t image_id) const {
        PackedByteArray result;
        if (image_id < 0 || image_id > std::numeric_limits<uint32_t>::max()) {
            return result;
        }
        const auto id = static_cast<uint32_t>(image_id);
        uint32_t length = 0;
        if (ofe_v2_image(token, id, nullptr, 0, &length) != OFE_LIMIT || length == 0 ||
            result.resize(length) != 0) {
            return {};
        }
        if (ofe_v2_image(token, id, result.ptrw(), length, &length) != OFE_OK) {
            return {};
        }
        return result;
    }

    ~OFEMatch() override {
        if (token != 0) {
            const int32_t status = ofe_v2_destroy(token);
            ERR_FAIL_COND_MSG(status != OFE_OK, "OFE match cleanup failed");
        }
    }
};

static void initialize(ModuleInitializationLevel level) {
    if (level == MODULE_INITIALIZATION_LEVEL_SCENE) {
        GDREGISTER_CLASS(OFEBattle);
        GDREGISTER_CLASS(OFEMatch);
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
