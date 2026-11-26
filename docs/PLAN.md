# Plan de trabajo y supervisión

Objetivo del proyecto: una implementación de ImGui en Rust, compatible **en conducta** con Dear
ImGui y con la API Rust como superficie principal. Sin ABI de C++.

Este documento existe para que puedas auditar el trabajo sin leer código: define el protocolo, las
puertas de salida y qué decisiones necesito de ti.

## 1. Protocolo de supervisión

- **Una rama y un PR por unidad de trabajo.** Nada va directo a `master`; los PRs llevan el informe
  de evidencia y tú los fusionas (o me dices que lo haga).
- **Regla dura de evidencia.** Ninguna afirmación de compatibilidad sin un número producido por
  `rust/tools/oracle` o un test que lo cite. Si no hay número, se escribe "provisional".
- **Informe por PR** con esta tabla, siempre igual:

  | Afirmación | Evidencia | Archivo |
  | --- | --- | --- |
  | p. ej. "IDs idénticos" | test `hashes_match_oracle` + 12 vectores del oráculo | `rust/.../id.rs` |

  más: "cómo verificarlo en 2 minutos" (comandos), riesgos y qué quedó sin probar.
- **Nada de "verde" a medias:** `cargo fmt --check`, `cargo clippy --all-targets -D warnings` y
  `cargo test` deben pasar antes de abrir el PR.
- **Parada obligatoria** cuando toque una decisión del apartado 3, cuando el oráculo contradiga al
  plan, o cuando un cambio afecte compromisos públicos (README, FORK, ROADMAP).

## 2. Fases y puertas de salida

Cada fase termina con una demostración ejecutable, no con una descripción.

### F0 — Gobernanza (en curso)
- CI que corra fmt/clippy/test en cada push (`.github/workflows/`); requiere que habilites Actions
  en el fork.
- ADR 0001 con el objetivo de compatibilidad y el commit de upstream pinneado.
- Puerta: primer PR con informe de evidencia y CI en verde.

### F1 — Arnés de conformidad (lo que hace medible todo lo demás)
- Formato de escenario (JSON): frames, IO, secuencia de llamadas.
- Driver C++: corre el escenario contra el árbol de referencia y vuelca draw lists a JSON.
- Driver Rust + comparador: mismas estructuras, diff campo a campo.
- Prueba negativa: un escenario manipulado a 1 ULP debe **fallar**; un arnés que no puede fallar no
  demuestra nada.
- Puerta: escenario de rectángulo + línea + círculo + texto con diff idéntico, y la prueba negativa
  en rojo.

### F2 — Núcleo
- `Context`, IO, ventanas, pila de IDs, clip stack, primitivas de layout, `ImDrawList` completo.
- Puerta: paridad de draw lists para las primitivas de F1 y para una ventana vacía con posición
  recordada entre frames.

### F3 — Widgets
- botón, label, checkbox, slider, drag, input de texto, combo, árbol, scroll, tooltip, popup, tabla.
- Puerta: una familia de widgets por PR, cada una con sus escenarios y paridad.

### F4 — Fuentes y texto
- Atlas, rasterizado, wrapping, offsets UTF-8 (como la referencia, no como Rust idiomático).
- Puerta: geometría de texto idéntica para los mismos atlas y cadenas (incluido emoji).

### F5 — Docking y backends
- Docking desde `upstream/docking`; después GLFW+OpenGL3 y winit+wgpu.
- Puerta: un ejemplo de upstream funciona sin cambios de código, salvo el backend.

### F6 — Otros lenguajes (solo si lo pides)
- ABI C + bindings. No está planificado ni prometido.

## 3. Decisiones que necesito de ti

| # | Decisión | Recomendación |
| --- | --- | --- |
| D1 | Ergonomía de la API: fiel a ImGui (`begin`/`end`, `ui.button`) o idiomática | **decidido: núcleo fiel** (ver abajo) |
| D2 | Publicar en crates.io (`oxidized-imgui` está libre hoy) | sí, cuando F2 cierre |
| D3 | Política con upstream: pin por fase y merge al inicio de fase | sí |
| D4 | Fuentes: portar `stb_truetype` (paridad) o usar un crate (rapidez) | portar, por paridad |
| D5 | Backend primero: GLFW+OpenGL3 o winit+wgpu | GLFW+GL3 (más simple de comparar) |
| D6 | Alcance del v1: qué widgets entran | lista de F3 menos tablas |
| D7 | Credenciales git: `gh auth login` una vez, o llave SSH temporal por sesión | `gh auth login` (ver abajo) |
| D8 | Cadencia: revisar cada PR o por fase | cada PR |

### Sobre D1

**Decidido: el port es fiel a ImGui.** La superficie pública sigue la forma de la API de ImGui
(`begin`/`end`, `begin_menu`, `input_text` con buffer), no una envoltura idiomática: lo que dibuja
sale del mismo grafo de llamadas que el C++, porque la paridad byte a byte es el contrato.

El objetivo de producto es que **un proyecto Rust que hoy usa bindings de ImGui pueda cambiar a
Oxidized-ImGui sin reescribir su interfaz**. Eso fija dos cosas:

- el núcleo mantiene los nombres y el orden de llamadas de ImGui, y
- la comodidad de los bindings (las capas tipo `imgui-rs`) se construye **encima** como capa de
  compatibilidad, no dentro del núcleo.

Todo widget nuevo se implementa primero en la forma de ImGui, aunque la capa idiomática acabe
ocultándola después.

### Sobre D7

Hoy no hay credenciales de git en la máquina, así que publico con una deploy key temporal que creo
y borro en la misma sesión. Funciona, pero es frágil. Lo limpio es que ejecutes una vez:

```powershell
winget install --id GitHub.cli
gh auth login
```

y desde entonces los pushes y los PRs son normales, sin llaves temporales.

## 4. Riesgos y mitigación

| Riesgo | Mitigación |
| --- | --- |
| Alcance desbordado (ImGui es enorme) | puertas de fase; nada fuera de fase sin tu aprobación |
| Afirmaciones sin respaldo | regla dura de evidencia + CI + prueba negativa del arnés |
| Deriva del upstream | commit pinneado por fase; merge solo al inicio de fase |
| Optimizar antes de tiempo | el rendimiento se mide, no se persigue en v1 |
| Licencia y marca | avisos MIT intactos; sin logo ni nombre ajenos |
| Docking/tablas como agujero negro | fases propias, sin fechas prometidas |

## 5. Qué no haré sin preguntarte

- Editar archivos fuera de `rust/` (el árbol de upstream es intocable).
- Cambiar compromisos públicos en README/FORK/ROADMAP.
- Añadir dependencias externas al crate.
- Force-push, borrar ramas o cambiar topics/descripción del repo.
- Instalar software de sistema.
- Tocar repos que no sean este fork.

## 6. Cómo verificar un PR en dos minutos

```powershell
cd "D:\Espacio de trabajo\oxidized-imgui\rust"
cargo test            # contrato
rust\tools\oracle\build.bat   # los números que el contrato afirma
```

Si un número del informe no aparece en la salida del oráculo, el informe está mal.
