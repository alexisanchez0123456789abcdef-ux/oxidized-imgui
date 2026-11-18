//! Oxidized ImGui: an implementation of ImGui in Rust, compatible with Dear ImGui.
//!
//! # Status
//!
//! Pre-alpha. What exists today is the smallest slice that can be checked against the reference:
//!
//! - [`id`] — `ImHashStr`/`ImHashData`, bit-identical to `imgui.cpp` (CRC32c, `###` resets the
//!   seed). Verified against values printed by the C++ reference.
//! - [`draw_list`] — `ImDrawVert`/`ImDrawCmd`/`TexRef` with the same layout the C++ compiler
//!   produces (sizes and offsets pinned by tests).
//! - [`context`] — frame counting and the identifier stack (`get_id`, `push_id`, `pop_id`).
//!
//! Geometry generation is provisional; matching the reference's draw lists is roadmap M1.
//!
//! # Compatibility contract
//!
//! The C++ tree in this repository is the specification. A piece of this crate is "compatible"
//! only when a test proves it against that tree: `rust/tools/oracle` builds the reference and
//! prints what it does, and the tests here assert against those numbers.

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]

pub mod context;
pub mod draw_list;
pub mod font;
pub mod id;
pub mod io;
pub mod style;

/// Matches `IMGUI_VERSION` in `imgui.h` of the reference this crate is checked against.
pub const IMGUI_VERSION: &str = "1.93.1 WIP";

/// Matches `IMGUI_VERSION_NUM`.
pub const IMGUI_VERSION_NUM: u32 = 19303;

pub use context::{Context, Window, WindowFlags};
pub use draw_list::{
    DrawCmd, DrawList, DrawVert, ImDrawIdx, TexRef, Vec2, draw_flags, path_bezier_cubic_to,
};
pub use font::{Font, Glyph};
pub use id::{ImGuiID, hash_data, hash_str, hash_str_utf8};
pub use io::{Io, Key, KeyData};
pub use style::{Col, Style};
