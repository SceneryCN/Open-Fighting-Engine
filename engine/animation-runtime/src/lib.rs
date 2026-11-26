// SPDX-License-Identifier: MPL-2.0
//! Load OFA 1.0 clips from a package and sample presentation frames one tick at a time.
use ofe_resource_runtime::{ResourceStore, json_integer};
use serde::{Deserialize, Deserializer};
use std::sync::Arc;

/// Frame origin, in image pixels.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    /// Horizontal anchor.
    #[serde(deserialize_with = "json_integer::read")]
    pub x: i32,
    /// Vertical anchor.
    #[serde(deserialize_with = "json_integer::read")]
    pub y: i32,
}

/// Rectangle in source image pixels, measured from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// One OFA frame. Frames which use one image share the same encoded resource bytes.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Number of presentation ticks.
    pub duration: u32,
    /// Anchor in pixels.
    pub origin: Origin,
    /// Image path relative to the OFA document.
    pub image_path: String,
    /// Encoded PNG image.
    pub image: Arc<[u8]>,
    /// Image region for this frame.
    pub source_rect: SourceRect,
    /// Event IDs in declaration order, emitted on entering this frame.
    pub events: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    format_version: String,
    id: String,
    #[serde(rename = "loop")]
    looping: bool,
    source: Source,
    frames: Vec<RawFrame>,
}

fn present_string<'de, D: Deserializer<'de>>(de: D) -> Result<Option<String>, D::Error> {
    String::deserialize(de).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    kind: String,
    #[serde(default, deserialize_with = "present_string")]
    image: Option<String>,
    #[serde(default, deserialize_with = "json_integer::optional")]
    columns: Option<i32>,
    #[serde(default, deserialize_with = "json_integer::optional")]
    rows: Option<i32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFrame {
    #[serde(deserialize_with = "json_integer::read")]
    duration: i32,
    origin: Origin,
    events: Vec<String>,
    #[serde(default, deserialize_with = "present_string")]
    image: Option<String>,
    #[serde(default, deserialize_with = "json_integer::optional")]
    index: Option<i32>,
}

fn identifier(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

/// Inspect the PNG header without decoding or allocating a pixel surface.
fn dimensions(path: &str, data: &[u8]) -> Result<(u32, u32), String> {
    const HEADER: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";
    if data.len() < 33 || !data.starts_with(HEADER) {
        return Err(format!("{path}: missing a complete PNG IHDR"));
    }
    let width = u32::from_be_bytes(data[16..20].try_into().expect("IHDR width"));
    let height = u32::from_be_bytes(data[20..24].try_into().expect("IHDR height"));
    if !(1..=i32::MAX as u32).contains(&width) || !(1..=i32::MAX as u32).contains(&height) {
        return Err(format!("{path}: invalid PNG dimensions {width} x {height}"));
    }
    let legal_depth = match data[25] {
        0 => matches!(data[24], 1 | 2 | 4 | 8 | 16),
        2 | 4 | 6 => matches!(data[24], 8 | 16),
        3 => matches!(data[24], 1 | 2 | 4 | 8),
        _ => false,
    };
    if !legal_depth || data[26] != 0 || data[27] != 0 || data[28] > 1 {
        return Err(format!("{path}: invalid PNG IHDR encoding"));
    }
    Ok((width, height))
}

struct Sheet {
    path: String,
    image: Arc<[u8]>,
    columns: u32,
    rows: u32,
    cell_width: u32,
    cell_height: u32,
}

impl Sheet {
    fn rect(&self, index: i32) -> Result<SourceRect, String> {
        let index = u32::try_from(index).map_err(|_| "negative sheet index".to_string())?;
        let row = index / self.columns;
        if row >= self.rows {
            return Err(format!("sheet index {index} outside grid"));
        }
        Ok(SourceRect {
            x: index % self.columns * self.cell_width,
            y: row * self.cell_height,
            width: self.cell_width,
            height: self.cell_height,
        })
    }
}

fn read_image(
    store: &mut ResourceStore,
    path: &str,
    reference: &str,
) -> Result<(Arc<[u8]>, u32, u32), String> {
    let image = store.read(path, reference)?;
    let (w, h) = dimensions(reference, &image)?;
    Ok((image, w, h))
}

impl Source {
    fn load(self, store: &mut ResourceStore, path: &str) -> Result<Option<Sheet>, String> {
        match (self.kind.as_str(), self.image, self.columns, self.rows) {
            ("sequence", None, None, None) => Ok(None),
            ("sheet", Some(image_path), Some(columns), Some(rows)) if columns > 0 && rows > 0 => {
                let (image, width, height) = read_image(store, path, &image_path)?;
                let (columns, rows) = (columns as u32, rows as u32);
                if width % columns != 0 || height % rows != 0 {
                    return Err(format!(
                        "image {image_path}: {width} x {height} is not divisible by grid {columns} x {rows}"
                    ));
                }
                Ok(Some(Sheet {
                    path: image_path,
                    image,
                    columns,
                    rows,
                    cell_width: width / columns,
                    cell_height: height / rows,
                }))
            }
            _ => Err("invalid OFA source kind or fields".to_string()),
        }
    }
}

/// Validated immutable animation clip. It owns shared references to its source images.
#[derive(Debug, Clone)]
pub struct Clip {
    id: String,
    looping: bool,
    frames: Vec<Frame>,
}

impl Clip {
    /// Load an OFA document given its package-root-relative path and all referenced images.
    pub fn load(store: &mut ResourceStore, path: &str) -> Result<Self, String> {
        let data = store.read("manifest.json", path)?;
        let result = Self::decode(store, path, &data);
        result.map_err(|e| format!("{path}: {e}"))
    }

    fn decode(store: &mut ResourceStore, path: &str, data: &[u8]) -> Result<Self, String> {
        let doc: Document = serde_json::from_slice(data).map_err(|e| e.to_string())?;
        if doc.format != "OFA" || doc.format_version != "1.0" {
            return Err("expected OFA format_version 1.0".into());
        }
        if !identifier(&doc.id) || doc.frames.is_empty() {
            return Err("invalid animation ID or empty frames".into());
        }
        let sheet = doc.source.load(store, path)?;
        let mut frames = Vec::with_capacity(doc.frames.len());
        for (i, raw) in doc.frames.into_iter().enumerate() {
            if raw.duration < 1 || raw.events.iter().any(|s| !identifier(s)) {
                return Err(format!("frames/{i}: invalid duration or event ID"));
            }
            let (image_path, image, source_rect) = match (&sheet, raw.index, raw.image) {
                (Some(sheet), Some(index), None) => (
                    sheet.path.clone(),
                    Arc::clone(&sheet.image),
                    sheet
                        .rect(index)
                        .map_err(|e| format!("frames/{i}/index: {e}"))?,
                ),
                (None, None, Some(reference)) => {
                    let (image, width, height) = read_image(store, path, &reference)
                        .map_err(|e| format!("frames/{i}/image: {e}"))?;
                    (
                        reference,
                        image,
                        SourceRect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                    )
                }
                _ => {
                    return Err(format!(
                        "frames/{i}: expected sequence image or sheet index matching source"
                    ));
                }
            };
            frames.push(Frame {
                duration: raw.duration as u32,
                origin: raw.origin,
                image_path,
                image,
                source_rect,
                events: raw.events,
            });
        }
        Ok(Self {
            id: doc.id,
            looping: doc.looping,
            frames,
        })
    }

    /// The OFA animation ID.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Frames in presentation order.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }
    /// Whether playback repeats after the final frame.
    pub fn is_looping(&self) -> bool {
        self.looping
    }
}

/// A player borrowing a clip for its lifetime.
pub type Player<'a> = Playback<&'a Clip>;

/// Tick-driven playback that retains either a borrowed clip or a shared handle.
#[derive(Debug, Clone)]
pub struct Playback<C> {
    clip: C,
    index: usize,
    spent: u32,
    finished: bool,
}

impl<C: std::borrow::Borrow<Clip>> Playback<C> {
    /// Begin with the first frame visible. Its events arrive on the first tick.
    pub fn new(clip: C) -> Self {
        Self {
            clip,
            index: 0,
            spent: 0,
            finished: false,
        }
    }
    /// Currently displayed frame.
    pub fn frame_index(&self) -> usize {
        self.index
    }
    /// True once a non-looping final frame consumes its full duration.
    pub fn finished(&self) -> bool {
        self.finished
    }
    /// Consume a time unit and return this frame's entry events.
    pub fn tick(&mut self) -> &[String] {
        let clip = self.clip.borrow();
        if self.finished {
            return &[];
        }
        if self.spent == clip.frames[self.index].duration {
            self.index = (self.index + 1) % clip.frames.len();
            self.spent = 0;
        }
        let frame = &clip.frames[self.index];
        self.spent += 1;
        self.finished =
            !clip.looping && self.index + 1 == clip.frames.len() && self.spent == frame.duration;
        if self.spent == 1 { &frame.events } else { &[] }
    }
}
