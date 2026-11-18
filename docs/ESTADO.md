# Estado del proyecto

_Actualizado: 2026-11-18. Generado desde `docs/progress.json`: no editar a mano._

## Reescritura a Rust: **39.6 %**

Del trabajo de reimplementación, esto es lo que está hecho **y demostrado**. El total del proyecto, contando la infraestructura que lo verifica, va al 45.0 %.

Qué cuenta como hecho: solo lo que tiene una prueba que lo compara con el código C++ original.
Un cambio de estado sin evidencia no sube el porcentaje.

| Área | Peso | Avance |
| --- | ---: | ---: |
| Núcleo: contexto, IO, ventanas, identificadores, layout | 31 | 55 % |
| Listas de dibujo y primitivas | 16 | 88 % |
| Widgets | 23 | 30 % |
| Fuentes y texto | 13 | 15 % |
| Docking y viewports | 10 | 0 % |
| Backends y ejemplos | 8 | 0 % |
| Infraestructura de verificación (no es la reescritura) | 10 | 100 % |

## Verificado

- **Hash de identificadores (CRC32c, reinicio con ###)** — id::tests::hashes_match_oracle (12 vectores del oráculo)
- **Pila de identificadores (get/push/pop)** — context::tests::ids_nest_like_the_reference
- **Ventana con posición y tamaño explícitos, memoria entre frames** — golden window_layout
- **Cursor y avance de ítems (Dummy, SameLine, rects de ítem)** — golden window_layout + context::tests
- **Ratón: posición redondeada, delta, clic, release, doble clic** — golden io_mouse_events
- **Tiempo: DeltaTime y reloj interno (float, como la referencia)** — golden io_mouse_events (doble clic depende del reloj)
- **Teclado: cola de eventos, duplicados, duraciones y modificadores** — golden io_keyboard_events
- **Fondo de ventana y orden del recorte** — golden window_bg
- **Pila de recorte y cortes de comando** — golden clip_stack
- **Estructuras con el layout exacto de C++** — draw_list::tests::layouts_match_reference
- **PrimRectUV con paridad exacta** — golden prim_rect_uv + prueba negativa
- **PrimRect con el UV del píxel blanco fijado** — golden prim_rect
- **Formas alineadas a píxel: AddRectFilled sin redondeo y AddLineH** — golden rect_filled y line_h
- **Relleno AA de rectángulo (miters)** — golden aa_rect
- **AA: rectángulo redondeado (arcos de esquina)** — golden aa_round_rect
- **AA: líneas de cualquier grosor** — golden aa_line
- **AA: uniones en bisel (esquinas agudas)** — golden aa_bevel
- **AA: segmentación automática de arcos** — golden arc_steps (radios 1..14)
- **AA: puntos de bézier por subdivisión** — golden bezier_points
- **AA: trazado de polilínea (miters, extremos abiertos)** — golden aa_polyline
- **AA: trazo cerrado con trazo interior (bordes)** — golden aa_border
- **Estilo: colores, redondeos y bordes por defecto** — golden style_defaults
- **Texto dibujado en la lista (geometría, sin UVs)** — golden text_draw (geometría; UVs omitidos por no ser reproducibles)
- **Botón: geometría, hover, estado activo y valor de retorno** — golden button
- **Botón dibujado: relleno por estado, etiqueta y UVs** — golden button_draw
- **Label y checkbox: rectángulo y alternancia** — golden widgets_basic
- **Slider: región de agarre, arrastre y redondeo** — golden slider_basic
- **UVs de texto con el atlas fijado por fixture** — golden text_draw con UVs + fixture text_draw.glyphs.txt
- **Medida de texto (CalcTextSize) con tabla de avances fijada** — golden font_metrics
- **Oráculo de constantes y layouts desde el C++** — rust/tools/oracle
- **Comparador de draw lists con golden y prueba negativa** — rust/tools/conformance + tests/golden
- **CI que corre fmt, clippy y tests** — .github/workflows/oxidized-rust.yml

## Lo siguiente, por peso

- Ciclo de frame completo: validación de IO y reset de draw data (peso 4)
- Pila de ventanas: colapso, scroll, foco, orden, auto-tamaño (peso 4)
- Layout completo: wrap, columnas, alineación, grupos (peso 4)

## Cómo se actualiza

`docs/progress.json` es la fuente de la verdad. Cambiar un `done` obliga a escribir su evidencia;
el CI regenera esta página y falla si no coincide, así que el porcentaje no puede separarse de la realidad.
