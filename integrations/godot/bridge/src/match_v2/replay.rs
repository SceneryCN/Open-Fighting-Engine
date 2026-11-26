// SPDX-License-Identifier: MPL-2.0
use super::*;
use ofe_replay_runtime::{MAX_BYTES, MAX_FRAMES, Replay};

#[derive(Default)]
pub(super) struct Tape {
    session: Option<Replay>,
    recording: bool,
    cursor: Option<usize>,
    error: String,
}

impl Tape {
    pub(super) fn busy(&self) -> bool {
        self.recording || self.cursor.is_some()
    }

    pub(super) fn check_live(&self, frame: u64) -> Result<(), i32> {
        if self.cursor.is_some() {
            return Err(INPUT_CONFLICT);
        }
        if self.recording
            && let Some(tape) = &self.session
        {
            if tape.len() >= MAX_FRAMES {
                return Err(LIMIT);
            }
            if frame != tape.len() as u64 + 1 {
                return Err(INPUT_CONFLICT);
            }
        }
        Ok(())
    }

    pub(super) fn record(&mut self, arena: &Arena, commands: &[Command]) -> Result<(), i32> {
        if self.recording {
            self.session
                .as_mut()
                .ok_or(INTERNAL)?
                .record(arena, commands)
                .map_err(|e| {
                    self.error = e.to_string();
                    INTERNAL
                })?;
        }
        Ok(())
    }
}

impl Match {
    fn replace_arena(&mut self, arena: Arena) {
        self.arena = arena;
        for actor in &mut self.actors {
            actor.active = actor.fallback;
            actor.player = Playback::new(Arc::clone(&actor.clips[actor.active]));
            actor.has_events = false;
            actor.damage = 0;
        }
        self.refresh();
    }

    fn playback_start(&mut self, loaded: Option<Replay>) -> Result<(), String> {
        let tape = loaded
            .as_ref()
            .or(self.tape.session.as_ref())
            .ok_or("No recording")?;
        if tape.is_empty() || tape.len() > 36000 {
            return Err("Training requires a recording of 1 to 36000 frames".into());
        }
        let arena = tape.initial_arena().map_err(|e| e.to_string())?;
        if arena.frame() != 0 || arena.config() != self.arena.config() {
            return Err("Select the original fighters and rules to play this recording".into());
        }
        if let Some(replay) = loaded {
            replay.verify().map_err(|e| e.to_string())?;
            self.tape.session = Some(replay);
        }
        self.replace_arena(arena);
        self.tape.recording = false;
        self.tape.cursor = Some(0);
        Ok(())
    }

    fn playback_step(&mut self) -> Result<(), String> {
        let index = self.tape.cursor.ok_or("Playback is not active")?;
        let tape = self.tape.session.as_ref().ok_or("No recording")?;
        let commands = tape.commands(index).ok_or("End of recording")?;
        let frame = self.arena.frame() + 1;
        // Commit only verified frames; a divergence leaves the last matching frame visible.
        let mut next = self.arena.clone();
        next.step(frame, commands)
            .map_err(|e| format!("Frame {frame}: {e:?}"))?;
        tape.check_frame(index, &next).map_err(|e| e.to_string())?;
        self.tape.cursor = (index + 1 < tape.len()).then_some(index + 1);
        self.arena = next;
        self.after_step();
        Ok(())
    }
}

fn training(handle: u64, action: impl FnOnce(&mut Match) -> Result<(), String>) -> i32 {
    guarded(|| {
        let mut registry = MATCHES.lock().map_err(|_| INTERNAL)?;
        let m = registry.active.get_mut(&handle).ok_or(INVALID_HANDLE)?;
        if !m.training {
            return Err(INVALID_ARGUMENT);
        }
        m.tape.error.clear();
        action(m).map_err(|message| {
            m.tape.error = message;
            INVALID_ARGUMENT
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_training_reset(handle: u64) -> i32 {
    training(handle, |m| {
        let arena = Arena::with_seed(m.arena.config().clone(), m.arena.seed())
            .map_err(|e| format!("{e:?}"))?;
        m.replace_arena(arena);
        m.tape.recording = false;
        m.tape.cursor = None;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_record_begin(handle: u64) -> i32 {
    training(handle, |m| {
        if m.arena.frame() != 0 || m.tape.busy() {
            return Err("Recording requires an idle frame-zero match".into());
        }
        m.tape.session = Some(Replay::new(&m.arena));
        m.tape.recording = true;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_record_end(handle: u64) -> i32 {
    training(handle, |m| {
        m.tape.recording = false;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_play_begin(handle: u64) -> i32 {
    training(handle, |m| m.playback_start(None))
}

#[unsafe(no_mangle)]
pub extern "C" fn ofe_v2_play_step(handle: u64) -> i32 {
    training(handle, |m| {
        let result = m.playback_step();
        if result.is_err() {
            m.tape.cursor = None;
        }
        result
    })
}

/// # Safety
/// data contains length readable bytes for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v2_play_load(handle: u64, data: *const u8, length: u32) -> i32 {
    if data.is_null() || length == 0 || length as usize > MAX_BYTES {
        return INVALID_ARGUMENT;
    }
    training(handle, |m| {
        // SAFETY: length bounded above; readable bytes are guaranteed by caller.
        let bytes = unsafe { std::slice::from_raw_parts(data, length as usize) };
        let tape = Replay::from_bytes(bytes).map_err(|e| e.to_string())?;
        m.playback_start(Some(tape))
    })
}

/// # Safety
/// Output follows the ofe_v2_snapshot caller-owned buffer contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v2_replay_export(
    handle: u64,
    out: *mut u8,
    capacity: u32,
    length: *mut u32,
) -> i32 {
    guarded(|| {
        let guard = MATCHES.lock().map_err(|_| INTERNAL)?;
        let m = guard.active.get(&handle).ok_or(INVALID_HANDLE)?;
        let replay = m
            .tape
            .session
            .as_ref()
            .filter(|r| !r.is_empty())
            .ok_or(INVALID_ARGUMENT)?;
        let bytes = replay.to_bytes().map_err(|_| LIMIT)?;
        // SAFETY: caller supplies exclusive aligned buffers.
        unsafe { copy(&bytes, out, capacity, length) }
    })
}

/// # Safety
/// Output follows the ofe_v2_snapshot caller-owned buffer contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ofe_v2_replay_info(
    handle: u64,
    out: *mut u8,
    capacity: u32,
    length: *mut u32,
) -> i32 {
    guarded(|| {
        let guard = MATCHES.lock().map_err(|_| INTERNAL)?;
        let m = guard.active.get(&handle).ok_or(INVALID_HANDLE)?;
        let bytes = serde_json::to_vec(
            &json!({"frames":m.tape.session.as_ref().map_or(0, Replay::len),
            "infinite_health":m.arena.infinite_health(), "error":m.tape.error}),
        )
        .map_err(|_| INTERNAL)?;
        // SAFETY: caller supplies exclusive aligned buffers.
        unsafe { copy(&bytes, out, capacity, length) }
    })
}
