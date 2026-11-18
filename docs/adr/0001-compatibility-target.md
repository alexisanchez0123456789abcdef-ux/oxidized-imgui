# ADR 0001 — objetivo de compatibilidad

- Fecha: 2026-11-18
- Estado: aceptado
- Decidido por: el propietario (confirmación explícita), redactado por el agente

## Contexto

Hay que fijar qué significa "compatible con Dear ImGui" antes de escribir más código, porque cada
definición lleva a una arquitectura distinta. El árbol C++ de este repositorio (fork, `master` en
`77fe4588`) es la única especificación ejecutable.

## Decisión

1. **Conducta primero.** Compatible significa: para la misma secuencia de llamadas, la misma
   geometría, visibilidad y draw data. Se comprueba con escenarios que corren en ambas
   implementaciones y se comparan campo a campo.
2. **La superficie principal es una crate de Rust** (`oxidized-imgui`), con la forma de ImGui
   (`begin`/`end`, `ui.button`), no una API nueva ni un wrapper del C++.
3. **No hay ABI de C++ ni reemplazo binario.** Rust no puede reproducir símbolos manglados, el
   layout de `ImGuiIO` ni los tipos `std::` del límite.
4. **Otros lenguajes quedan para después**, solo si aparece una necesidad real, mediante una ABI C.
   No está planificado ni prometido (D6).
5. **Pin de upstream por fase**: se fusiona `upstream/master` (y `upstream/docking` cuando toque)
   al inicio de cada fase, no continuamente (D3).
6. **Fuentes**: se portará `stb_truetype` (paridad) en lugar de usar una crate externa, porque un
   rasterizador distinto cambia las métricas de los glifos y rompe la paridad de draw lists (D4).
7. **Primer backend**: GLFW + OpenGL3, por ser el más fácil de comparar (D5).
8. **Publicación**: la crate se publica en crates.io como `oxidized-imgui` (nombre libre a fecha de
   este ADR) cuando cierre la fase F2 (D2).
9. **Revisión**: cada PR se revisa con su informe de evidencia (D8).
10. **Ergonomía (D1)**: el núcleo es fiel a ImGui; una capa idiomática, si se hace, va encima y no
    contamina el núcleo.

## Consecuencias

- Todo lo que no se pueda comparar contra el árbol C++ no se anuncia como compatible.
- Los números de compatibilidad viven en tests que citan la salida del oráculo
  (`rust/tools/oracle`, `rust/tools/conformance`).
- El fork no puede desengancharse de la red de ocornut mientras se use como oráculo; es una ventaja,
  no un límite.
- La paridad estricta limita elecciones idiomáticas: por ejemplo, el texto se indexa en bytes UTF-8
  como la referencia, no por `char` de Rust.
