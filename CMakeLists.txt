cmake_minimum_required(VERSION 3.22)
project(OpenFightingEngine LANGUAGES C CXX)
include(FetchContent)

# Pin the stable binding generator and select its Godot 4.7 API.
set(GODOT_CPP_REV 507ed9d840c01a3c5b2a39af8bb4000bfac30bf5)
set(GODOTCPP_API_VERSION "4.7" CACHE STRING "Godot extension API")

set(GODOTCPP_BUILD_PROFILE "${CMAKE_CURRENT_SOURCE_DIR}/scripts/build/godot-profile.json" CACHE FILEPATH "OFE bindings profile")
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
FetchContent_Declare(godot-cpp
    GIT_REPOSITORY https://github.com/godotengine/godot-cpp.git
    GIT_TAG ${GODOT_CPP_REV})
FetchContent_MakeAvailable(godot-cpp)

find_program(CARGO cargo REQUIRED)
set(OFE_RUST_TARGET "" CACHE STRING "Rust target triple matching the C++ compiler")
if(NOT OFE_RUST_TARGET)
    find_program(RUSTC rustc REQUIRED)
    execute_process(COMMAND ${RUSTC} -vV OUTPUT_VARIABLE rust_version
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}" COMMAND_ERROR_IS_FATAL ANY)
    string(REGEX MATCH "host: ([^\n\r]+)" rust_host "${rust_version}")
    set(OFE_RUST_TARGET "${CMAKE_MATCH_1}")
endif()
if(MSVC)
    set(rust_filename ofe_bridge.lib)
else()
    set(rust_filename libofe_bridge.a)
endif()
set(rust_dir "${CMAKE_BINARY_DIR}/cargo")
set(rust_archive "${rust_dir}/${OFE_RUST_TARGET}/release/${rust_filename}")
# Cargo checks its own dependency graph every invocation, including build scripts.
add_custom_target(build_rust
    COMMAND ${CARGO} build --locked --release --manifest-path integrations/godot/bridge/Cargo.toml
        --target-dir "${rust_dir}" --target "${OFE_RUST_TARGET}"
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}" BYPRODUCTS "${rust_archive}" VERBATIM)

add_library(ofe_godot SHARED integrations/godot/extension/ofe_extension.cpp)
add_library(ofe_rust INTERFACE)
add_dependencies(ofe_rust build_rust)
target_link_libraries(ofe_rust INTERFACE "${rust_archive}")
target_include_directories(ofe_rust INTERFACE integrations/godot/bridge/include)
target_compile_features(ofe_godot PRIVATE cxx_std_17)
target_link_libraries(ofe_godot PRIVATE godot::cpp ofe_rust)
if(MSVC)
    target_compile_options(ofe_godot PRIVATE /W4)
else()
    target_compile_options(ofe_godot PRIVATE -Wall -Wextra -Wpedantic)
endif()
if(WIN32)
    set_target_properties(ofe_godot PROPERTIES PREFIX "")
    target_link_libraries(ofe_rust INTERFACE ws2_32 userenv bcrypt ntdll advapi32 kernel32 dbghelp synchronization)
    if(MINGW)
        target_link_options(ofe_godot PRIVATE -static -static-libgcc -static-libstdc++)
    endif()
elseif(APPLE)
    target_link_libraries(ofe_rust INTERFACE "-framework Security" "-framework CoreFoundation" "-liconv")
elseif(UNIX)
    target_link_libraries(ofe_rust INTERFACE dl pthread m)
endif()
install(TARGETS ofe_godot LIBRARY DESTINATION bin RUNTIME DESTINATION bin)

include(CTest)
if(BUILD_TESTING)
    add_executable(abi_smoke tests/integration/abi_smoke.c)
    target_link_libraries(abi_smoke PRIVATE ofe_rust)
    add_test(NAME abi_smoke COMMAND abi_smoke)
endif()

# Godot 4.7.2 crashes on editor shutdown after discovering the first extension
# during import. Register it before first boot, exactly as the editor does.
install(CODE [[
    set(extension_list "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/.godot/extension_list.cfg")
    if(NOT EXISTS "${extension_list}")
        get_filename_component(cache_dir "${extension_list}" DIRECTORY)
        file(MAKE_DIRECTORY "${cache_dir}")
        file(WRITE "${extension_list}" "res://ofe.gdextension\n")
    endif()
]])
