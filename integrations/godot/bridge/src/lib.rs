// SPDX-License-Identifier: MPL-2.0
//! OFE version 1 C ABI. The public C header defines the caller contract.
use ofe_combat_core::{Battle, Error};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{LazyLock, Mutex};

const OK: i32 = 0;
const INVALID_ARGUMENT: i32 = 1;
const INVALID_HANDLE: i32 = 2;
const INCOMPATIBLE: i32 = 3;
const INPUT_CONFLICT: i32 = 4;
const LIMIT: i32 = 5;
const INTERNAL: i32 = 6;

/// C-facing state. Only this versioned projection crosses the boundary.
#[repr(C)]
pub struct StateV1 {
    size: u32,
    version: u32,
    frame: u64,
    entity_id: u32,
    x: i32,
    z: i32,
    h: i32,
}

const _: () = assert!(size_of::<StateV1>() == 32);

#[derive(Default)]
struct Registry {
    last_token: u64,
    battles: HashMap<u64, Battle>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Mutex::default);

fn with_registry(operation: impl FnOnce(&mut Registry) -> Result<(), i32>) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let mut registry = REGISTRY.lock().map_err(|_| INTERNAL)?;
        operation(&mut registry)
    }))
    .unwrap_or(Err(INTERNAL))
    .err()
    .unwrap_or(OK)
}

fn core_error(error: Error) -> i32 {
    match error {
        Error::InvalidAxis => INVALID_ARGUMENT,
        Error::InvalidFrame | Error::DuplicateInput => INPUT_CONFLICT,
        Error::Overflow => LIMIT,
    }
}

/// Allocate a uniquely identified battle.
///
/// # Safety
/// A non-null output must point to aligned, exclusive writable token storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v1_create(out: *mut u64) -> i32 {
    if out.is_null() || !out.is_aligned() {
        return INVALID_ARGUMENT;
    }
    with_registry(|registry| {
        let token = registry.last_token.checked_add(1).ok_or(LIMIT)?;
        registry.battles.try_reserve(1).map_err(|_| LIMIT)?;
        registry.battles.insert(token, Battle::new());
        registry.last_token = token;
        // SAFETY: the caller guarantees valid exclusive output storage.
        unsafe { out.write(token) };
        Ok(())
    })
}

/// Release a live token. Releasing a stale token yields INVALID_HANDLE.
#[unsafe(no_mangle)]
pub extern "C" fn ofe_v1_destroy(handle: u64) -> i32 {
    with_registry(|registry| {
        registry.battles.remove(&handle).ok_or(INVALID_HANDLE)?;
        Ok(())
    })
}

/// Queue one axis input on a live battle.
#[unsafe(no_mangle)]
pub extern "C" fn ofe_v1_submit(handle: u64, frame: u64, axis: i32) -> i32 {
    with_registry(|registry| {
        registry
            .battles
            .get_mut(&handle)
            .ok_or(INVALID_HANDLE)?
            .submit(frame, axis)
            .map_err(core_error)
    })
}

/// Advance one fixed tick.
#[unsafe(no_mangle)]
pub extern "C" fn ofe_v1_step(handle: u64) -> i32 {
    with_registry(|registry| {
        registry
            .battles
            .get_mut(&handle)
            .ok_or(INVALID_HANDLE)?
            .step()
            .map_err(core_error)
    })
}

/// Copy the committed simulation state into a caller-owned v1 buffer.
///
/// # Safety
/// The caller supplies aligned, exclusive storage with an initialized size.
/// For sizes >= 32, the first 32 bytes must be writable and version initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v1_read(handle: u64, out: *mut StateV1) -> i32 {
    if out.is_null() || !out.is_aligned() {
        return INVALID_ARGUMENT;
    }
    // SAFETY: read only the initialized size prefix before checking capacity.
    let size = unsafe { out.cast::<u32>().read() };
    if size < size_of::<StateV1>() as u32 {
        return INCOMPATIBLE;
    }
    // SAFETY: at this size the version field must be initialized by the caller.
    if unsafe { (&raw const (*out).version).read() } != 1 {
        return INCOMPATIBLE;
    }
    with_registry(|registry| {
        let state = registry
            .battles
            .get(&handle)
            .ok_or(INVALID_HANDLE)?
            .snapshot();
        let output = StateV1 {
            size,
            version: 1,
            frame: state.frame,
            entity_id: state.entity_id,
            x: state.x,
            z: state.z,
            h: state.h,
        };
        // SAFETY: checked capacity, exclusive writable storage per contract.
        unsafe { out.write(output) };
        Ok(())
    })
}
