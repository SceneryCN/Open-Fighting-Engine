// SPDX-License-Identifier: MIT
//! Construct OFA 1.0 documents. Image references are relative to the OFA file.
use serde::Serialize;
use serde_json::{Value, json};

/// Timing and pixel anchor shared by sequence frames and sheet cells.
#[derive(Clone, Debug, Serialize)]
pub struct Frame<T> {
    #[serde(skip)]
    pub image: T,
    pub duration: i32,
    pub origin: Origin,
    pub events: Vec<String>,
}

/// Pixel coordinates relative to the frame's top-left corner.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Origin {
    pub x: i32,
    pub y: i32,
}

impl<T> Frame<T> {
    pub fn new(image: T, duration: i32, x: i32, y: i32) -> Self {
        Self {
            image,
            duration,
            origin: Origin { x, y },
            events: Vec::new(),
        }
    }
}

/// Source-specific frame types prevent a sheet index from being used as a file path.
pub enum Frames {
    Sequence(Vec<Frame<String>>),
    Sheet {
        image: String,
        columns: i32,
        rows: i32,
        frames: Vec<Frame<i32>>,
    },
}

pub struct Animation {
    pub id: String,
    pub looping: bool,
    pub frames: Frames,
}

fn encode_frames<T: Serialize>(frames: &[Frame<T>], key: &str) -> Result<Vec<Value>, String> {
    if frames.is_empty() {
        return Err("an animation needs at least one frame".into());
    }
    frames
        .iter()
        .map(|frame| {
            if frame.duration <= 0 {
                return Err("frame duration must be positive".into());
            }
            let mut data = json!(frame);
            data[key] = json!(frame.image);
            Ok(data)
        })
        .collect()
}

impl Animation {
    /// Build the JSON document. The format CLI checks IDs and referenced PNG resources on disk.
    pub fn document(&self) -> Result<Value, String> {
        let (source, frames) = match &self.frames {
            Frames::Sequence(frames) => {
                (json!({"kind":"sequence"}), encode_frames(frames, "image")?)
            }
            Frames::Sheet {
                image,
                columns,
                rows,
                frames,
            } => {
                let capacity = i64::from(*columns) * i64::from(*rows);
                if *columns <= 0
                    || *rows <= 0
                    || frames
                        .iter()
                        .any(|f| f.image < 0 || i64::from(f.image) >= capacity)
                {
                    return Err("invalid sheet dimensions or frame index".into());
                }
                (
                    json!({"kind":"sheet","image":image,"columns":columns,"rows":rows}),
                    encode_frames(frames, "index")?,
                )
            }
        };
        Ok(
            json!({"format":"OFA","format_version":"1.0","id":self.id,"loop":self.looping,"source":source,"frames":frames}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sheet_indices_must_fit_grid() {
        let clip = Animation {
            id: "test".into(),
            looping: false,
            frames: Frames::Sheet {
                image: "sheet.png".into(),
                columns: 2,
                rows: 1,
                frames: vec![Frame::new(2, 1, 0, 0)],
            },
        };
        assert!(clip.document().is_err());
    }
}
