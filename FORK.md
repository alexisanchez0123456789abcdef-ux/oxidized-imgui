# Fork notes

## Provenance

- Upstream: <https://github.com/ocornut/imgui> — MIT, Omar Cornut and contributors.
- Fork: <https://github.com/alexisanchez0123456789abcdef-ux/oxidized-imgui>
- Forked on: 2026-11-17.
- Base commit: `77fe4588e6af934b4167b91312174ca3fbb0d71a` — `master`, 2026-11-16.
- Branches copied: `master`, `docking`, and the upstream `features/*` and `external/*` branches.

Everything outside `rust/` is upstream material. Do not edit it in place: it changes only through
merges from upstream, so the reference stays comparable.

## Intention

Build **Oxidized ImGui**: an ImGui implementation written in Rust that is compatible with Dear
ImGui — the same immediate-mode model, widget semantics, identifier stack and layout rules,
exposed as a Rust crate.

The fork exists because Dear ImGui's C++ implementation *is* the specification. Behaviour that is
not written down anywhere is decided by that code, so the reference sits next to the port, builds,
and answers questions the Rust tests ask it.

## What "compatible" means here

1. **Behavioural** — for the same sequence of calls: the same geometry, visibility and draw data.
   The contract is the C++ tree; `rust/tools/oracle` builds it and the Rust tests assert what it
   prints.
2. **Rust API** — the crate is shaped like ImGui (same calls, same semantics, safe by default), so
   porting code is mechanical rather than a redesign.

Explicitly not goals: C++ ABI or binary compatibility, a drop-in replacement for an existing ImGui
build, or a C-compatible ABI for other languages in v1. Those can be revisited later if a real
need appears; they are not what this fork is for.

## Scope

In: context and IO, identifier stack, windows and widget stack, layout, clipping, draw lists,
widgets, tables; then fonts, docking and backends as the milestones reach them.

Out: ImPlot / ImNodes / ImGuizmo (separate upstream projects), demo-only and misc tooling, a new
styling system, a visual designer, remote/network UI.

## Sync policy

- An `upstream` remote points at `ocornut/imgui`.
- Before a milestone is declared done: fetch `upstream`, merge `upstream/master` (and
  `upstream/docking` once docking is in scope), resolve conflicts in favour of upstream, then
  re-run the conformance tests.
- Rust code lives in `rust/` only; upstream files stay byte-identical outside merge commits.

## Licensing and attribution

MIT. The upstream notice is kept in `LICENSE.txt`, and Rust sources derived from upstream keep
their original copyright lines. Renaming a project is allowed under MIT; removing the notice is
not. "Dear ImGui" and "ImGui" are the upstream project's names: Oxidized ImGui is independent, not
endorsed, and says so in the README and repository description.

## Decisions

Design questions that need a decision before their milestone live in
[`docs/ROADMAP.md`](docs/ROADMAP.md). Each closed decision gets an ADR under `docs/adr/`.
