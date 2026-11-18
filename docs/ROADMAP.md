# Oxidized ImGui — roadmap

Estado: pre-alpha. El plan y el protocolo de supervisión están en [`PLAN.md`](PLAN.md); este
documento lleva el estado de cada fase. Las decisiones cerradas están en
[`adr/0001-compatibility-target.md`](adr/0001-compatibility-target.md).

## F0 — Gobernanza (hecho)

- [x] CI en cada push y PR: `cargo fmt --check`, `clippy -D warnings`, tests
      (`.github/workflows/oxidized-rust.yml`).
- [x] ADR 0001 con el objetivo de compatibilidad y las decisiones D1–D10.
- [x] Upstream pinneado en `77fe4588` y política de merge al inicio de fase.

Puerta: primer PR con informe de evidencia y CI en verde.

## F1 — Arnés de conformidad (hecho, con cobertura mínima)

- [x] Formato de dump determinista: texto con floats como patrón de bits, LF, sin atlas ni
      decimales que dependan del formateador.
- [x] Driver C++ (`rust/tools/conformance/driver.cpp`) que corre el escenario contra el árbol de
      referencia y imprime el dump.
- [x] Golden generado por ese driver (`tests/golden/prim_rect_uv.txt`) y test de igualdad byte a
      byte en `cargo test`.
- [x] Prueba negativa: una mutación de un bit se detecta (primera diferencia en el byte 166).
- [x] Escenario cubierto: `PrimRectUV` (sin atlas, sin AA, sin redondeo).
- [ ] Ampliar escenarios: `AddRectFilled` con redondeo/AA/subpíxel, `AddLine`, `AddCircle`, texto.

Puerta cumplida para lo cubierto: dump idéntico y arnés capaz de fallar. El resto de primitivas es
F2, y hasta que cada una tenga su escenario se declara **provisional**.

## F2 — Núcleo

- [x] `ImHashStr` / `ImHashData`: CRC32c, seed y `###`, bit a bit.
- [x] `ImDrawVert`, `ImDrawCmd`, `TexRef`: tamaño y offsets del compilador C++.
- [x] Pila de identificadores (`get_id`, `push_id`, `pop_id`).
- [x] `PrimRectUV` con paridad de draw list.
- [ ] Contexto completo: IO, ventanas, clip stack, bookkeeping de frame.
- [ ] Primitivas restantes y sus escenarios.

## F3 — Widgets

Una familia por PR (botón, label, checkbox, slider, drag, input, combo, árbol, scroll, tooltip,
popup), cada una con sus escenarios y paridad.

## F4 — Fuentes y texto

Atlas, rasterizado con `stb_truetype`, wrapping, offsets en bytes UTF-8 como la referencia.

## F5 — Docking y backends

Docking desde `upstream/docking`; después GLFW+OpenGL3 y winit+wgpu.

## F6 — Otros lenguajes (no planificado)

ABI C y bindings, solo si aparece una necesidad real. No prometido.

## Decisiones abiertas

Ya no bloquean F0–F2; ver ADR 0001. Cualquier decisión nueva se añade como ADR antes de
implementarla.

## No-goals de v1

ImPlot / ImNodes / ImGuizmo, diseñador visual, UI remota, reemplazo de `egui`, ABI o
compatibilidad binaria de C++, reemplazo directo de un build de ImGui existente.
