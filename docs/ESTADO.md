# Estado del proyecto

_Actualizado: 2026-11-18. Generado desde `docs/progress.json`: no editar a mano._

## Reescritura a Rust: **61.7 %**

Del trabajo de reimplementación, esto es lo que está hecho **y demostrado**. El total del proyecto, contando la infraestructura que lo verifica, va al 64.3 %.

Qué cuenta como hecho: solo lo que tiene una prueba que lo compara con el código C++ original.
Un cambio de estado sin evidencia no sube el porcentaje.

| Área | Peso | Avance |
| --- | ---: | ---: |
| Núcleo: contexto, IO, ventanas, identificadores, layout | 58 | 76 % |
| Listas de dibujo y primitivas | 17 | 88 % |
| Widgets | 27 | 78 % |
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
- **Borde de ventana (marco alineado de dos anillos)** — golden window_border
- **Barra de título: banda, texto y orden de dibujo** — golden window_title
- **Iconos de ventana: flecha de colapso y cruz de cierre** — golden window_icons (flecha, cruz y texto desplazado por los iconos)
- **Iconos de ventana: hover, pulsación, colapso y cierre** — golden window_icons_states (14 estados: resalte, pulsado, soltar dentro y fuera, colapsada y expandida)
- **Arrastrar la ventana desde la flecha (umbral de arrastre y desplazamiento)** — golden window_icons_drag (8 estados con la posición de la ventana)
- **Trazo del borde agarrado: más grueso y en color separador, sobre el borde de la ventana** — golden window_resize_edges con lista de dibujo completa (29 776 bytes)
- **Doble clic en el tirador: ajusta la ventana a su contenido, con el mínimo aplicado** — golden window_resize_autofit (11 frames: clic simple y doble clic)
- **Cursor de redimensionado: flechas según el tirador, con la espera de 40 ms de los bordes** — golden window_resize_cursor (10 frames de cursor)
- **Tirador izquierdo y los cuatro bordes: el tamaño sale de la esquina opuesta fija** — golden window_resize_edges (16 frames: tamaño y posición)
- **Arrastrar el tirador de la esquina: el tamaño sale de la posición del ratón y su recorte** — golden window_resize_drag (10 frames con tamaño y posición)
- **Tiradores de redimensionado: el de abajo a la derecha siempre, el otro al pasar por encima, con sus tres colores** — golden window_resize_grips (8 frames: reposo, hover, pulsado y esquina izquierda)
- **Tamaño mínimo de ventana: recorta el tamaño pedido por API (32), con mínimo duro de 1 px para el auto-tamaño** — golden window_min_size (6 frames: 10, 31, 32, 120 y auto con tamaño pedido)
- **Barra por páginas: clic en la pista avanza una página, con Shift forzando la búsqueda absoluta** — golden window_scrollbar_page (16 frames: página, página otra vez, Shift y absoluto)
- **Barra de scroll horizontal: aparición, alto que quita al área interior, dibujo y arrastre** — golden window_hscrollbar (8 frames con contenido más ancho que la ventana)
- **Bloqueo de la ventana que rueda: el solapamiento no roba la rueda hasta superar el umbral** — golden window_wheel_lock (11 frames, dos ventanas solapadas 2 px)
- **Auto-tamaño con contenido cambiante: el tamaño sigue al contenido con un frame de retraso** — golden window_autosize_change (6 frames: encoger y crecer)
- **Auto-tamaño: la ventana toma el tamaño que pide su contenido** — golden window_autosize (6 frames con tamaño, área interior y contenido)
- **Rueda del ratón: paso de cinco tamaños de fuente y desplazamiento simétrico** — golden window_wheel (11 frames: dos muescas abajo, una arriba y media muesca)
- **Arrastrar el agarre y clicar la pista (búsqueda absoluta, con colores de hover y pulsado)** — golden window_scrollbar_drag (16 frames: hover, agarre, arrastre, clic en pista)
- **Barra de scroll vertical: aparición, ancho del área interior y dibujo del agarre** — golden window_scrollbar (5 frames con rectángulo de barra, área interior y volcado)
- **Scroll por API: máximo desde el contenido, desplazamiento del contenido y recorte** — golden window_scroll (10 frames con scroll, máximo, contenido y rectángulo interior)
- **Mover una ventana: arrastre desde la barra y desde el cuerpo, con recorte al salir de pantalla** — golden windows_move (15 estados con la posición de la ventana)
- **Pila de ventanas: orden de dibujo, hover entre ventanas y foco al hacer clic** — golden windows_order (9 frames con la lista de dibujo de cada ventana en orden)
- **Desplazamiento de índices por comando** — golden cmd_offsets
- **Cierre de ventana: recorte bajo la banda y limpieza de comandos** — goldens window_bg y window_title + sonda de comandos
- **Escena completa: ventana con título y botón, comparada entera** — golden scene_window_button
- **Pila de recorte y cortes de comando** — golden clip_stack
- **Estructuras con el layout exacto de C++** — draw_list::tests::layouts_match_reference
- **PrimRectUV con paridad exacta** — golden prim_rect_uv + prueba negativa
- **PrimRect con el UV del píxel blanco fijado** — golden prim_rect
- **Formas alineadas a píxel: AddRectFilled sin redondeo y AddLineH** — golden rect_filled y line_h
- **Relleno AA de rectángulo (miters)** — golden aa_rect
- **AA: rectángulo redondeado (arcos de esquina)** — golden aa_round_rect
- **AA: líneas de cualquier grosor** — golden aa_line
- **AA: trazos con la textura de líneas del atlas (remates AA, bisel recortado, trazo cerrado)** — golden aa_strokes + fixture aa_strokes.lineuvs.txt
- **AA: uniones en bisel (esquinas agudas)** — golden aa_bevel
- **AA: segmentación automática de arcos** — golden arc_steps (radios 1..14)
- **AA: puntos de bézier por subdivisión** — golden bezier_points
- **AA: trazado de polilínea (miters, extremos abiertos)** — golden aa_polyline
- **AA: trazo cerrado con trazo interior (bordes)** — golden aa_border
- **Estilo: colores, redondeos y bordes por defecto** — golden style_defaults
- **Texto dibujado en la lista (geometría, sin UVs)** — golden text_draw (geometría; UVs omitidos por no ser reproducibles)
- **Botón: geometría, hover, estado activo y valor de retorno** — golden button
- **Botón dibujado: relleno por estado, etiqueta y UVs** — golden button_draw
- **Botón: reposo, hover y activo comparados** — golden button_states
- **Botón: borde de marco (FrameBorderSize)** — golden button_border
- **Label y checkbox: rectángulo y alternancia** — golden widgets_basic
- **Slider: región de agarre, arrastre y redondeo** — golden slider_basic
- **Selectable: fila a todo el ancho, resalte por estado y clic** — golden selectable (8 frames, 38668 bytes)
- **Clic en una fila del popup: pulsación, cierre y foco del anfitrión** — golden popup_click (12 frames, 45697 bytes)
- **Combo desplegable: caja, flecha, popup colocado bajo la caja y filas** — golden combo (11 frames, 52130 bytes)
- **Cabecera colapsable: marco, flecha, hover/activo y alternar (etiqueta al soltar, flecha al pulsar)** — golden collapsing_header (12 frames, lista completa de 79125 bytes)
- **Nodos de árbol: fila estrecha, resaltado a todo el ancho, sangría y árbol anidado** — golden tree_node (13 frames, lista completa de 56344 bytes)
- **Región hija: lista de dibujo propia, recorte interior, borde en la lista del padre y avance del layout** — golden child_region (4 frames, dos listas cada uno, 22092 bytes)
- **Scroll de la región hija completo: barra en la lista del padre, rueda sobre la región y arrastre de su barra** — golden child_region (13 frames, dos listas, 122487 bytes)
- **Popup como ventana: lista propia, fondo de popup, medida en el primer frame y z-orden** — golden popup (10 frames, 48770 bytes)
- **Tooltip: ventana al ratón, sin ratón ni teclado, medida en el primer frame** — golden tooltip (6 frames, 19705 bytes)
- **Barra de menús: franja, recorte propio, layout horizontal y entradas** — golden menu_bar (5 frames, 23068 bytes)
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
