# Estado del proyecto

_Actualizado: 2026-11-18. Generado desde `docs/progress.json`: no editar a mano._

## Reescritura a Rust: **8.9 %**

Del trabajo de reimplementación, esto es lo que está hecho **y demostrado**. El total del proyecto, contando la infraestructura que lo verifica, va al 18.0 %.

Qué cuenta como hecho: solo lo que tiene una prueba que lo compara con el código C++ original.
Un cambio de estado sin evidencia no sube el porcentaje.

| Área | Peso | Avance |
| --- | ---: | ---: |
| Núcleo: contexto, IO, ventanas, identificadores, layout | 30 | 13 % |
| Listas de dibujo y primitivas | 8 | 50 % |
| Widgets | 22 | 0 % |
| Fuentes y texto | 12 | 0 % |
| Docking y viewports | 10 | 0 % |
| Backends y ejemplos | 8 | 0 % |
| Infraestructura de verificación (no es la reescritura) | 10 | 100 % |

## Verificado

- **Hash de identificadores (CRC32c, reinicio con ###)** — id::tests::hashes_match_oracle (12 vectores del oráculo)
- **Pila de identificadores (get/push/pop)** — context::tests::ids_nest_like_the_reference
- **Estructuras con el layout exacto de C++** — draw_list::tests::layouts_match_reference
- **PrimRectUV con paridad exacta** — golden prim_rect_uv + prueba negativa
- **PrimRect con el UV del píxel blanco fijado** — golden prim_rect
- **Formas alineadas a píxel: AddRectFilled sin redondeo y AddLineH** — golden rect_filled y line_h
- **Oráculo de constantes y layouts desde el C++** — rust/tools/oracle
- **Comparador de draw lists con golden y prueba negativa** — rust/tools/conformance + tests/golden
- **CI que corre fmt, clippy y tests** — .github/workflows/oxidized-rust.yml

## Lo siguiente, por peso

- Ciclo de frame y contexto completo (peso 6)
- Pila de ventanas y gestión de ventanas (peso 6)
- Entrada y salida: ratón, teclado, tiempo (peso 5)

## Cómo se actualiza

`docs/progress.json` es la fuente de la verdad. Cambiar un `done` obliga a escribir su evidencia;
el CI regenera esta página y falla si no coincide, así que el porcentaje no puede separarse de la realidad.
