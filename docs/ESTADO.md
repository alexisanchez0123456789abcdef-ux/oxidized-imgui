# Estado del proyecto

_Actualizado: 2026-11-26. Generado desde `docs/progress.json`: no editar a mano._

## Reescritura a Rust: **69.9 %**

Del trabajo de reimplementación, esto es lo que está hecho **y demostrado**. El total del proyecto, contando la infraestructura que lo verifica, va al 71.8 %.

Qué cuenta como hecho: solo lo que tiene una prueba que lo compara con el código C++ original.
Un cambio de estado sin evidencia no sube el porcentaje.

| Área | Peso | Avance |
| --- | ---: | ---: |
| Núcleo: contexto, IO, ventanas, identificadores, layout | 59 | 76 % |
| Listas de dibujo y primitivas | 18 | 100 % |
| Widgets | 38 | 97 % |
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
- **Teclado de texto: juego completo de ImGuiKey, cola de caracteres y repetición (delay/rate, DownDurationPrev)** — golden io_text_events (23 frames: cola, pulsación, repetición y suelta) + io::tests (5 casos)
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
- **Pila de ventanas: orden de dibujo, hover entre ventanas, foco al hacer clic, foco por API (SetNextWindowFocus), colapso por API (SetNextWindowCollapsed) posición por API (SetWindowPos, truncada) y tamaño por API (SetWindowSize, truncado)** — goldens windows_order (9 frames con la lista de dibujo de cada ventana en orden), windows_focus (7 frames con SetNextWindowFocus sobre A y B) windows_collapse_api (5 frames: colapsar y expandir por API, con los ítems del cuerpo saltados) windows_set_pos (5 frames: mover por nombre y que la posición se mantenga) y windows_set_size (5 frames: redimensionar por nombre y que el tamaño se mantenga)
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
- **AA: trazo cerrado con trazo interior (bordes) y AddRect sin redondeo (atajo alineado a píxel y caída a polilínea)** — goldens aa_border y rect_stroke
- **AA: círculos** — golden aa_circles (14 círculos rellenos y contorneados bajo el corte arc-fast, con la tabla de segmentos cacheada y el radio mínimo con alfa)
- **Estilo: colores, redondeos y bordes por defecto** — golden style_defaults
- **Texto dibujado en la lista (geometría, sin UVs)** — golden text_draw (geometría; UVs omitidos por no ser reproducibles)
- **Texto con CpuFineClip: glifos descartados fuera del recorte y quads y UVs recortados en el borde** — golden text_clip (5 recortes, 4279 bytes, con UVs)
- **Cortes de comando y recorte por comando** — golden cmd_split (3 comandos: el clip que no cambia no parte, el que cambia sí, y el push/pop sin geometría se fusiona)
- **InvisibleButton: ítem del tamaño pedido que no dibuja nada pero hovera y pulsa al soltar** — golden invisible_button (4 frames con el valor de retorno y la lista sin geometría propia)
- **Botón: geometría, hover, estado activo y valor de retorno** — golden button
- **Botón dibujado: relleno por estado, etiqueta y UVs** — golden button_draw
- **Botón: reposo, hover y activo comparados** — golden button_states
- **Botón: borde de marco (FrameBorderSize)** — golden button_border
- **Label y checkbox: rectángulo y alternancia** — golden widgets_basic, spacing, new_line, indent y cursor_pos (GetCursorPos/SetCursorPos en coordenadas de ventana)
- **BulletText: un solo ítem con el disco y el texto sangrado a FontSize + FramePadding.x*2** — golden bullet_text (dos entradas, con el rectángulo del ítem y la lista)
- **LabelText: valor en el ancho del ítem y etiqueta al lado con ItemInnerSpacing y FramePadding** — golden label_text (ítem con etiqueta y sin ella, con el rectángulo del ítem y la lista)
- **TextColored y TextDisabled: color explícito empaquetado con ColorConvertFloat4ToU32 y alfa del estilo** — golden text_colors (Text normal, TextColored en rojo, TextDisabled y Text después, con los colores en la lista)
- **ProgressBar: marco, parte rellena con PlotHistogram, overlay de porcentaje tras el relleno y ancho por defecto que llena el área de contenido** — golden progress_bar (4 barras: 25%, 75%, overlay propio y ancho por defecto, con el rectángulo del ítem y la lista)
- **Bullet: disco AA de FontSize*0.2, ítem cuadrado con el tamaño de la fuente y SameLine que sangra la entrada** — golden bullet (ítem, cursor tras el bullet y el texto en la misma línea, y lista completa)
- **BeginDisabled/EndDisabled: Style::Alpha por DisabledAlpha (0.6) en todos los colores y ítems que no hoveran ni pulsan** — golden disabled (4 frames: reposo, pulsación y suelta sobre el botón deshabilitado, y hover sobre el siguiente, con los valores de retorno)
- **Separador: horizontal en layout vertical, vertical en la barra de menús, y SeparatorText con etiqueta entre dos líneas** — goldens separator (ítem, cursor y lista), separator_text (ítem, cursor y líneas con centro sesgado), separator_text_empty (una sola línea sin etiqueta) y menu_bar_separator (la forma vertical: alto de la línea de la barra)
- **Slider: región de agarre, arrastre y redondeo** — golden slider_basic
- **Campo de texto: marco, texto recortado al marco, cursor desde el clic y liberación al clicar fuera** — golden input_text (10 frames, 36202 bytes) + input_text::tests (4 casos)
- **Edición por teclado: escribir, flechas, Home/End, Delete/Backspace, selección con Shift, resalte y scroll que sigue al cursor** — golden input_text_edit (21 frames, 123667 bytes)
- **Portapapeles y validación: Ctrl+A/C/X/V, Escape que revierte y Enter que aplica, con escritura en vivo al buffer del llamador** — golden input_text_clipboard (25 frames) + input_text::tests
- **Selección con el ratón (arrastre y doble clic por palabra) y deshacer/rehacer con la pila completa de stb_textedit** — golden input_text_mouse_undo (22 frames) + input_text::tests
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
- **Columnas de menú: anchos por columna acumulados en el frame y bloqueados desde el anterior, espaciado solo entre columnas que existen** — menu_columns::tests (6 casos) + golden submenu
- **Submenús desplegables: ventana hija propia bajo la barra, filas a todo el ancho con etiqueta y atajo en sus columnas, cierre al elegir y foco del anfitrión** — golden submenu (11 frames, lista completa de 72305 bytes)
- **Tablas, primer corte: columnas fijas, filas que crecen con su contenido o con la altura mínima pedida, relleno y espaciado de celda, índices agrupados por columna como el splitter, cabeceras (banda TableHeaderBg, nombres en sus celdas y su fila contada por TableGetRowIndex), bandas alternas de RowBg, fondo de celda por TableSetBgColor, la rejilla interior de BordersInner, el borde exterior de BordersOuter con el atajo alineado de AddRect las columnas stretch de SizingStretchSame y con pesos explícitos de SizingStretchProp, las celdas direccionadas con TableSetColumnIndex la API pública de columnas (TableGetColumnCount/Name) las etiquetas de cabecera cortadas en ##, el recorte de las celdas a su columna, los comandos por columna de TableMergeDrawChannels el ancho automático por contenido de WidthAuto el fondo de una cabecera fuera de su fila los flags derivados de cada columna (TableGetColumnFlags) las bandas de RowBg tras una fila de cabecera el avance de TableNextColumn, que abre fila al pasar la última columna, el resalte de la cabecera bajo el ratón su estado pulsado (HeaderActive) TableGetHoveredColumn/Row el peso derivado del contenido de SizingStretchProp el fondo de fila explícito de TableSetBgColor TableNextColumn abriendo fila sin TableNextRow el segundo fondo de celda que sustituye al primero el bit IsHovered de la columna bajo el ratón las especificaciones de orden por defecto (TableGetSortSpecs) el caso SortTristate, que empieza ordenado por la primera columna ordenable, el bit IsSorted que apunta a la columna ordenada, la flecha de orden que la cabecera dibuja (triángulo AA con su número de orden y la dirección descendente), el clic de cabecera que alterna la dirección (ascendente, descendente y otra vez ascendente, con SpecsDirty puesto hasta que el llamante lo limpia), el arrastre de una cabecera que reordena las columnas (orden de display, aplicado en el BeginTable siguiente), el Shift+clic que añade una segunda columna al orden (SortMulti, con su número de orden), la ventana hija interior de una tabla con ScrollY (lista de dibujo propia, recorte contra el host y el fondo de fila con el clip del host) y la API de consulta después de cerrar la tabla** — goldens tables_basic, tables_row_height, tables_headers, tables_rowbg, tables_cellbg, tables_borders, tables_borders_outer, tables_stretch, tables_stretch_weights, tables_set_column, tables_names, tables_header_id, tables_clip, tables_auto_width, tables_header_inrow, tables_column_flags, tables_rowbg_headers, tables_next_column, tables_header_hover, tables_header_active, tables_hovered, tables_stretch_prop, tables_row_bg_color, tables_column_bounds, tables_cell_bg_twice, tables_column_hovered_flag, tables_sort_specs, tables_sort_tristate, tables_sort_arrow, tables_sort_click, tables_reorder, tables_sort_multi, tables_scroll_child, tables_column_sorted_flag y tables_after_end
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
