// SPDX-License-Identifier: MPL-2.0
//! Versioned offline recordings shared by the simulator and the game.
use ofe_combat_core::arena::{Arena, ArenaSnapshot, Command};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum serialized file size accepted by both writer and reader.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
/// One hour of 60 Hz input. The training UI has its own shorter recording limit.
pub const MAX_FRAMES: usize = 216_000;
const FORMAT: &str = "OFE_REPLAY";

/// Invalid data, failed playback or the first mismatched simulation state.
#[derive(Debug)]
pub enum ReplayError {
    Invalid(String),
    Mismatch {
        frame: u64,
        expected: u64,
        actual: u64,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Mismatch {
                frame,
                expected,
                actual,
            } => write!(
                f,
                "frame {frame}: state hash mismatch: expected {expected:016x}, actual {actual:016x}"
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

mod hex64 {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{value:016x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 16
            || !s
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(D::Error::custom("expected 16 lowercase hexadecimal digits"));
        }
        u64::from_str_radix(&s, 16).map_err(D::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedFrame {
    frame: u64,
    commands: Vec<Command>,
    #[serde(with = "hex64")]
    hash: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    initial: ArenaSnapshot,
    #[serde(with = "hex64")]
    initial_hash: u64,
    frames: Vec<RecordedFrame>,
}

/// Records the authoritative checkpoint and subsequent committed inputs.
pub struct Replay {
    doc: Document,
    initial_frame: u64,
}

fn check(arena: &Arena, frame: u64, expected: u64) -> Result<(), ReplayError> {
    if arena.frame() != frame {
        return Err(ReplayError::Invalid(format!(
            "expected frame {frame}, got {}",
            arena.frame()
        )));
    }
    let actual = arena.state_hash();
    if actual != expected {
        return Err(ReplayError::Mismatch {
            frame,
            expected,
            actual,
        });
    }
    Ok(())
}

impl Replay {
    /// Start a recording at any committed frame, including inside an action.
    pub fn new(arena: &Arena) -> Self {
        Self {
            initial_frame: arena.frame(),
            doc: Document {
                format: FORMAT.into(),
                version: 1,
                initial: arena.checkpoint(),
                initial_hash: arena.state_hash(),
                frames: Vec::new(),
            },
        }
    }

    pub fn len(&self) -> usize {
        self.doc.frames.len()
    }
    pub fn is_empty(&self) -> bool {
        self.doc.frames.is_empty()
    }

    /// Append after a successful simulation step. Rejected attempts leave the tape intact.
    pub fn record(&mut self, after: &Arena, commands: &[Command]) -> Result<(), ReplayError> {
        if self.len() >= MAX_FRAMES {
            return Err(ReplayError::Invalid("recording frame limit reached".into()));
        }
        let last = self
            .doc
            .frames
            .last()
            .map_or(self.initial_frame, |f| f.frame);
        if last.checked_add(1) != Some(after.frame()) || commands.len() > after.fighters().len() {
            return Err(ReplayError::Invalid(format!(
                "invalid recording append at frame {}",
                after.frame()
            )));
        }
        self.doc.frames.push(RecordedFrame {
            frame: after.frame(),
            commands: commands.to_vec(),
            hash: after.state_hash(),
        });
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplayError> {
        let bytes =
            serde_json::to_vec(&self.doc).map_err(|e| ReplayError::Invalid(e.to_string()))?;
        if bytes.len() > MAX_BYTES {
            return Err(ReplayError::Invalid(
                "recording exceeds file size limit".into(),
            ));
        }
        Ok(bytes)
    }

    /// Parse the envelope, restore and check its initial hash, and validate frame order.
    /// Frame hashes are compared as the caller actually simulates the commands.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplayError> {
        if bytes.len() > MAX_BYTES {
            return Err(ReplayError::Invalid(
                "recording exceeds file size limit".into(),
            ));
        }
        let doc: Document = serde_json::from_slice(bytes)
            .map_err(|e| ReplayError::Invalid(format!("invalid replay: {e}")))?;
        if doc.format != FORMAT || doc.version != 1 {
            return Err(ReplayError::Invalid(format!(
                "unsupported replay format {} version {}",
                doc.format, doc.version
            )));
        }
        if doc.frames.len() > MAX_FRAMES {
            return Err(ReplayError::Invalid(
                "recording frame limit exceeded".into(),
            ));
        }
        let arena = Arena::from_checkpoint(&doc.initial)
            .map_err(|e| ReplayError::Invalid(format!("invalid initial checkpoint: {e:?}")))?;
        let initial_frame = arena.frame();
        check(&arena, initial_frame, doc.initial_hash)?;
        let mut previous = initial_frame;
        for tick in &doc.frames {
            if previous.checked_add(1) != Some(tick.frame)
                || tick.commands.len() > arena.fighters().len()
            {
                return Err(ReplayError::Invalid(format!(
                    "invalid input record after frame {previous}"
                )));
            }
            previous = tick.frame;
        }
        Ok(Self { doc, initial_frame })
    }

    pub fn initial_arena(&self) -> Result<Arena, ReplayError> {
        Arena::from_checkpoint(&self.doc.initial)
            .map_err(|e| ReplayError::Invalid(format!("invalid initial checkpoint: {e:?}")))
    }

    pub fn check_initial(&self, arena: &Arena) -> Result<(), ReplayError> {
        check(arena, self.initial_frame, self.doc.initial_hash)
    }

    /// Zero-based index in the recording, independent of the initial simulation frame.
    pub fn commands(&self, index: usize) -> Option<&[Command]> {
        self.doc.frames.get(index).map(|f| f.commands.as_slice())
    }

    pub fn check_frame(&self, index: usize, after: &Arena) -> Result<(), ReplayError> {
        let entry = self
            .doc
            .frames
            .get(index)
            .ok_or_else(|| ReplayError::Invalid(format!("no replay entry {index}")))?;
        check(after, entry.frame, entry.hash)
    }

    /// Restore the embedded rules and state and stop at the earliest divergent tick.
    pub fn verify(&self) -> Result<Arena, ReplayError> {
        let mut arena = self.initial_arena()?;
        self.check_initial(&arena)?;
        for (index, entry) in self.doc.frames.iter().enumerate() {
            arena.step(entry.frame, &entry.commands).map_err(|e| {
                ReplayError::Invalid(format!(
                    "frame {}: simulation rejected input: {e:?}",
                    entry.frame
                ))
            })?;
            self.check_frame(index, &arena)?;
        }
        Ok(arena)
    }
}
