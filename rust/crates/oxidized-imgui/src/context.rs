//! Context and identifier stack.
//!
//! Mirrors the part of the reference that decides *which widget is which*: the context owns the
//! ID stack, `get_id` hashes a label with the current stack top as the seed, and `push_id` seeds a
//! nested scope. Windows, layout and interaction are not implemented yet (roadmap M1/M3).

use crate::id::{ImGuiID, hash_str_utf8};

/// Frame lifecycle and identifier stack.
#[derive(Debug, Default)]
pub struct Context {
    id_stack: Vec<ImGuiID>,
    frame_count: u64,
    last_frame_draw_calls: usize,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a frame. The reference also validates the IO and resets the draw data.
    pub fn begin_frame(&mut self) {
        self.frame_count += 1;
    }

    /// Ends a frame.
    pub fn end_frame(&mut self) {}

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Identifier of the current scope, `0` at the root like the reference's window-less case.
    pub fn current_id(&self) -> ImGuiID {
        self.id_stack.last().copied().unwrap_or(0)
    }

    /// Hash of `label` in the current scope (`ImGui::GetID`).
    pub fn get_id(&self, label: &str) -> ImGuiID {
        hash_str_utf8(label, self.current_id())
    }

    /// Opens a scope seeded by `label` (`ImGui::PushID`).
    pub fn push_id(&mut self, label: &str) {
        let id = self.get_id(label);
        self.id_stack.push(id);
    }

    /// Closes the innermost scope (`ImGui::PopID`).
    pub fn pop_id(&mut self) {
        self.id_stack.pop();
    }

    pub fn id_stack_depth(&self) -> usize {
        self.id_stack.len()
    }

    pub fn last_frame_draw_calls(&self) -> usize {
        self.last_frame_draw_calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_nest_like_the_reference() {
        let mut ctx = Context::new();
        assert_eq!(ctx.current_id(), 0);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", 0));

        ctx.push_id("Window");
        let window_id = hash_str_utf8("Window", 0);
        assert_eq!(ctx.current_id(), window_id);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", window_id));

        ctx.push_id("Child");
        let child_id = hash_str_utf8("Child", window_id);
        assert_eq!(ctx.current_id(), child_id);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", child_id));

        ctx.pop_id();
        ctx.pop_id();
        assert_eq!(ctx.id_stack_depth(), 0);
        assert_eq!(ctx.current_id(), 0);
    }

    #[test]
    fn frame_counter_advances() {
        let mut ctx = Context::new();
        ctx.begin_frame();
        ctx.end_frame();
        ctx.begin_frame();
        assert_eq!(ctx.frame_count(), 2);
    }
}
