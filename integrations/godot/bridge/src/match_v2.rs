// SPDX-License-Identifier: MPL-2.0
//! Two-character Arena ABI. Simulation and presentation live behind the same handle.
use super::{INPUT_CONFLICT, INTERNAL, INVALID_ARGUMENT, INVALID_HANDLE, LIMIT, OK};
use ofe_animation_runtime::{Clip, Playback};
use ofe_character_runtime::{Character, duel_scene};
use ofe_combat_core::{
    arena::{Arena, ArenaEvent, ArenaFighter, Command},
    duel::{Controls, DuelError, Event, State},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    sync::{Arc, LazyLock, Mutex},
};

const SCENE: &str = include_str!("../../../../tools/ofe-sim/fixtures/scene.json");
const ADVANCED: &str = include_str!("../../../../tools/ofe-sim/fixtures/arena_scene.json");

fn state_name(state: State) -> &'static str {
    match state {
        State::Idle => "Idle",
        State::Walk => "Walk",
        State::Jump => "Jump",
        State::Fall => "Fall",
        State::LaneSwitch => "LaneSwitch",
        State::Attack => "Attack",
        State::Guard => "Guard",
        State::Hitstun => "Hitstun",
        State::Blockstun => "Blockstun",
        State::Ko => "Ko",
    }
}

fn event_json(event: &ArenaEvent) -> Value {
    match *event {
        ArenaEvent::Base(Event::Moved { id, position: p }) => {
            json!({"kind":"Moved","id":id,"position":{"x":p.x,"z":p.z,"h":p.h}})
        }
        ArenaEvent::Base(Event::StateChanged { id, state }) => {
            json!({"kind":"StateChanged","id":id,"state":state_name(state)})
        }
        ArenaEvent::Base(Event::Hit {
            attacker,
            target,
            damage,
        }) => json!({"kind":"Hit","attacker":attacker,"target":target,"damage":damage}),
        ArenaEvent::Base(Event::Blocked { attacker, target }) => {
            json!({"kind":"Blocked","attacker":attacker,"target":target})
        }
        ArenaEvent::Base(Event::Knockout { id }) => json!({"kind":"Knockout","id":id}),
        ArenaEvent::Started { id, move_index } => {
            json!({"kind":"Started","id":id,"move_index":move_index})
        }
        ArenaEvent::Cancelled { id, from, into } => {
            json!({"kind":"Cancelled","id":id,"from":from,"into":into})
        }
        ArenaEvent::Dashed { id } => json!({"kind":"Dashed","id":id}),
        ArenaEvent::Armored { attacker, target } => {
            json!({"kind":"Armored","attacker":attacker,"target":target})
        }
        ArenaEvent::ProjectileSpawned { id, owner } => {
            json!({"kind":"ProjectileSpawned","id":id,"owner":owner})
        }
    }
}

struct Actor {
    name: String,
    max_hp: i32,
    max_energy: i32,
    clips: Vec<Arc<Clip>>,
    active: usize,
    fallback: usize,
    player: Playback<Arc<Clip>>,
    has_events: bool,
}

impl Actor {
    fn new(character: Character) -> Self {
        let clips: Vec<_> = character.animations.into_iter().map(Arc::new).collect();
        let fallback = clips
            .iter()
            .position(|clip| clip.id() == "idle")
            .unwrap_or(0);
        Self {
            name: character.name,
            max_hp: character.combat.attributes.max_health,
            max_energy: character.combat.attributes.max_energy,
            player: Playback::new(Arc::clone(&clips[fallback])),
            clips,
            active: fallback,
            fallback,
            has_events: false,
        }
    }

    fn tick(&mut self, state: State) {
        let active = self
            .clips
            .iter()
            .position(|clip| clip.id().eq_ignore_ascii_case(state_name(state)))
            .unwrap_or(self.fallback);
        if active != self.active {
            self.active = active;
            self.player = Playback::new(Arc::clone(&self.clips[active]));
        }
        self.has_events = !self.player.tick().is_empty();
    }

    fn snapshot(&self, slot: &ArenaFighter, image_ids: &HashMap<usize, u32>) -> Value {
        let f = slot.fighter;
        let clip = &self.clips[self.active];
        let index = self.player.frame_index();
        let frame = &clip.frames()[index];
        let rect = frame.source_rect;
        let events: &[String] = if self.has_events { &frame.events } else { &[] };
        json!({"id":f.id,"team":slot.team,"name":self.name,"x":f.position.x,"z":f.position.z,"h":f.position.h,
            "facing":f.facing,"state":state_name(f.state),"hp":f.health,"max_hp":self.max_hp,
            "energy":slot.energy,"max_energy":self.max_energy,"animation":{"clip":clip.id(),"index":index,
            "image_id":image_ids[&(frame.image.as_ptr() as usize)],"rect":[rect.x,rect.y,rect.width,rect.height],
            "origin":[frame.origin.x,frame.origin.y],"events":events}})
    }
}

struct Match {
    arena: Arena,
    actors: [Actor; 2],
    images: Vec<Arc<[u8]>>,
    image_ids: HashMap<usize, u32>,
    snapshot: Vec<u8>,
}

impl Match {
    fn open(first: &Path, second: &Path) -> Result<Self, i32> {
        let a = Character::load(first).map_err(|_| INVALID_ARGUMENT)?;
        let b = Character::load(second).map_err(|_| INVALID_ARGUMENT)?;
        let config = duel_scene::bind([&a.combat, &b.combat], SCENE, ADVANCED)
            .map_err(|_| INVALID_ARGUMENT)?;
        let arena = Arena::new(config).map_err(|_| INVALID_ARGUMENT)?;
        let actors = [Actor::new(a), Actor::new(b)];
        let mut images = Vec::new();
        let mut image_ids = HashMap::new();
        for actor in &actors {
            for frame in actor.clips.iter().flat_map(|clip| clip.frames()) {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    image_ids.entry(frame.image.as_ptr() as usize)
                {
                    let id = u32::try_from(images.len()).map_err(|_| LIMIT)?;
                    images.push(Arc::clone(&frame.image));
                    entry.insert(id);
                }
            }
        }
        let mut instance = Self {
            arena,
            actors,
            images,
            image_ids,
            snapshot: Vec::new(),
        };
        instance.refresh();
        Ok(instance)
    }

    fn refresh(&mut self) {
        let fighters: Vec<_> = self
            .actors
            .iter()
            .zip(self.arena.fighters())
            .map(|(actor, slot)| actor.snapshot(slot, &self.image_ids))
            .collect();
        let events: Vec<_> = self.arena.events().iter().map(event_json).collect();
        let projectiles:Vec<_>=self.arena.projectiles().iter().map(|p|
            json!({"id":p.id,"owner":p.owner,"x":p.position.x,"z":p.position.z,"h":p.position.h})).collect();
        self.snapshot.clear();
        serde_json::to_writer(
            &mut self.snapshot,
            &json!({"frame":self.arena.frame(),"fighters":fighters,
            "events":events,"projectiles":projectiles}),
        )
        .expect("JSON value can be written to Vec");
    }

    fn step(&mut self, frame: u64, p1: u32, p2: u32) -> Result<(), i32> {
        let commands = [command(1, p1)?, command(2, p2)?];
        self.arena.step(frame, &commands).map_err(|e| match e {
            DuelError::WrongFrame => INPUT_CONFLICT,
            DuelError::Overflow => LIMIT,
            DuelError::InvalidInput | DuelError::InvalidConfig => INVALID_ARGUMENT,
        })?;
        for (actor, slot) in self.actors.iter_mut().zip(self.arena.fighters()) {
            actor.tick(slot.fighter.state);
        }
        self.refresh();
        Ok(())
    }
}

fn command(id: u32, bits: u32) -> Result<Command, i32> {
    let move_index = ((bits >> 8) & 0xffff).checked_sub(1).map(|v| v as usize);
    let attack = bits & 16 != 0;
    if bits & !0x00ff_ff7f != 0 || bits & 3 == 3 || (attack && move_index.is_some()) {
        return Err(INVALID_ARGUMENT);
    }
    Ok(Command {
        id,
        move_index,
        dash: bits & 64 != 0,
        controls: Controls {
            axis: i32::from(bits & 2 != 0) - i32::from(bits & 1 != 0),
            jump: bits & 4 != 0,
            switch_lane: bits & 8 != 0,
            attack,
            guard: bits & 32 != 0,
        },
    })
}

#[derive(Default)]
struct Matches {
    count: u64,
    active: HashMap<u64, Match>,
}
static MATCHES: LazyLock<Mutex<Matches>> = LazyLock::new(Mutex::default);

fn guarded(operation: impl FnOnce() -> Result<(), i32>) -> i32 {
    catch_unwind(AssertUnwindSafe(operation))
        .unwrap_or(Err(INTERNAL))
        .err()
        .unwrap_or(OK)
}

/// Create a match from two local OFC directories.
/// # Safety
/// Paths are live NUL-terminated strings. Output is exclusive aligned token storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v2_create(p1: *const c_char, p2: *const c_char, out: *mut u64) -> i32 {
    if p1.is_null() || p2.is_null() || out.is_null() || !out.is_aligned() {
        return INVALID_ARGUMENT;
    }
    guarded(|| {
        // SAFETY: paths stay valid throughout this call.
        let a = unsafe { CStr::from_ptr(p1) }
            .to_str()
            .map_err(|_| INVALID_ARGUMENT)?;
        let b = unsafe { CStr::from_ptr(p2) }
            .to_str()
            .map_err(|_| INVALID_ARGUMENT)?;
        if a.is_empty() || b.is_empty() {
            return Err(INVALID_ARGUMENT);
        }
        let instance = Match::open(Path::new(a), Path::new(b))?;
        let mut registry = MATCHES.lock().map_err(|_| INTERNAL)?;
        let count = registry
            .count
            .checked_add(1)
            .filter(|c| *c < super::V2_BIT)
            .ok_or(LIMIT)?;
        let id = count | super::V2_BIT;
        registry.active.try_reserve(1).map_err(|_| LIMIT)?;
        registry.active.insert(id, instance);
        registry.count = count;
        // SAFETY: output is exclusive and aligned per caller contract.
        unsafe { out.write(id) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_destroy(handle: u64) -> i32 {
    guarded(|| {
        let mut matches = MATCHES.lock().map_err(|_| INTERNAL)?;
        matches.active.remove(&handle).ok_or(INVALID_HANDLE)?;
        if matches.active.is_empty() {
            matches.active.shrink_to_fit();
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_step(handle: u64, frame: u64, p1: u32, p2: u32) -> i32 {
    guarded(|| {
        MATCHES
            .lock()
            .map_err(|_| INTERNAL)?
            .active
            .get_mut(&handle)
            .ok_or(INVALID_HANDLE)?
            .step(frame, p1, p2)
    })
}

unsafe fn copy(bytes: &[u8], out: *mut u8, capacity: u32, length: *mut u32) -> Result<(), i32> {
    if length.is_null() || !length.is_aligned() || (out.is_null() && capacity != 0) {
        return Err(INVALID_ARGUMENT);
    }
    let size = u32::try_from(bytes.len()).map_err(|_| LIMIT)?;
    // SAFETY: caller supplies exclusive aligned length storage.
    unsafe { length.write(size) };
    if capacity < size {
        return Err(LIMIT);
    }
    if size != 0 {
        // SAFETY: checked capacity and non-null; caller guarantees disjoint writable storage.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
    }
    Ok(())
}

/// Read the currently committed frame JSON without advancing it.
/// # Safety
/// The caller provides disjoint writable output and aligned length storage.
/// NULL output with zero capacity is a length query.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v2_snapshot(
    handle: u64,
    out: *mut u8,
    capacity: u32,
    length: *mut u32,
) -> i32 {
    guarded(|| {
        let guard = MATCHES.lock().map_err(|_| INTERNAL)?;
        let instance = guard.active.get(&handle).ok_or(INVALID_HANDLE)?;
        // SAFETY: passed through to copy under the same caller requirements.
        unsafe { copy(&instance.snapshot, out, capacity, length) }
    })
}

/// Read encoded PNG bytes indexed by a match-local image ID.
/// # Safety
/// Output and length requirements match ofe_v2_snapshot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v2_image(
    handle: u64,
    image_id: u32,
    out: *mut u8,
    capacity: u32,
    length: *mut u32,
) -> i32 {
    guarded(|| {
        let guard = MATCHES.lock().map_err(|_| INTERNAL)?;
        let instance = guard.active.get(&handle).ok_or(INVALID_HANDLE)?;
        let image = instance
            .images
            .get(image_id as usize)
            .ok_or(INVALID_ARGUMENT)?;
        // SAFETY: passed through to copy under the same caller requirements.
        unsafe { copy(image, out, capacity, length) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr::null_mut;

    fn fixture(character: &str) -> CString {
        CString::new(format!(
            "{}/../../../tools/ofe-sim/fixtures/{character}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn snapshot(token: u64) -> Vec<u8> {
        let mut length = 0;
        // SAFETY: null,0 is a size query. The second call owns exactly length bytes.
        unsafe {
            assert_eq!(ofe_v2_snapshot(token, null_mut(), 0, &mut length), LIMIT);
            let mut bytes = vec![0; length as usize];
            assert_eq!(
                ofe_v2_snapshot(token, bytes.as_mut_ptr(), length, &mut length),
                OK
            );
            bytes
        }
    }

    #[test]
    fn published_frames_and_image_buffers_are_atomic_until_a_valid_step() {
        let p1 = fixture("amber");
        let p2 = fixture("indigo");
        let mut token = 0;
        // SAFETY: temporary paths stay alive and the token is an aligned local.
        assert_eq!(
            unsafe { ofe_v2_create(p1.as_ptr(), p2.as_ptr(), &mut token) },
            OK
        );
        assert_eq!(super::super::ofe_v1_destroy(token), INVALID_HANDLE);
        let initial = snapshot(token);
        let parsed: Value = serde_json::from_slice(&initial).unwrap();
        assert_eq!(parsed["fighters"][0]["hp"], 60);
        assert_eq!(parsed["fighters"][1]["hp"], 75);
        let image = parsed["fighters"][0]["animation"]["image_id"]
            .as_u64()
            .unwrap() as u32;
        let mut sentinel = 37;
        let mut length = 0;
        // SAFETY: the buffer capacity is one byte, the length is independently writable.
        assert_eq!(
            unsafe { ofe_v2_image(token, image, &mut sentinel, 1, &mut length) },
            LIMIT
        );
        assert!(length > 8);
        assert_eq!(sentinel, 37);
        for mask in [3, 128, 0x0100_0000, (1 << 8) | 16, 2 << 8] {
            assert_eq!(ofe_v2_step(token, 1, 16, mask), INVALID_ARGUMENT);
            assert_eq!(snapshot(token), initial);
        }
        assert_eq!(ofe_v2_step(token, 2, 16, 0), INPUT_CONFLICT);
        assert_eq!(ofe_v2_step(token, 1, 16, 0), OK);
        assert_eq!(ofe_v2_step(token, 2, 0, 0), OK);
        let after: Value = serde_json::from_slice(&snapshot(token)).unwrap();
        assert_eq!(after["fighters"][1]["hp"], 60);
        assert!(
            after["events"]
                .as_array()
                .unwrap()
                .contains(&json!({"kind":"Hit","attacker":1,"target":2,"damage":15}))
        );
        assert_eq!(ofe_v2_destroy(token), OK);
        assert_eq!(ofe_v2_step(token, 3, 0, 0), INVALID_HANDLE);
    }
}
