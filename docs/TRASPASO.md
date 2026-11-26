# Traspaso: cómo continuar este proyecto

Este documento existe para que **una sesión nueva** pueda seguir el trabajo sin haber
leído la conversación anterior. Si eres un agente: lee esto primero, luego `docs/PLAN.md`.

## Qué es el proyecto

Una reescritura en **Rust** de **Dear ImGui 1.93.1 WIP** (upstream `ocornut/imgui`,
commit base `77fe4588e6af934b4167b91312174ca3fbb0d71a`), con **paridad de comportamiento**:
lo que dibuja el port debe ser **idéntico byte a byte** a lo que dibuja el C++ original.

No hay ABI de C++. El port es Rust puro y el C++ sólo se usa como **referencia** para
comparar.

## Dónde está y cómo se trabaja

- Repositorio y carpeta de trabajo: `D:\Espacio de trabajo\oxidized-imgui`
- Sólo se toca **`rust/`** y **`docs/`**. El árbol C++ de upstream es **intocable**.
- El arnés de conformidad está en `rust/tools/conformance/`.

### Protocolo (resumen; el completo en `docs/PLAN.md`)

1. **Una rama + una PR por unidad.** Nada de mezclar unidades.
2. **CI verde antes de fusionar** (el trabajo `fmt, clippy, tests`).
3. **Squash merge**, luego borrar la rama y dejar `master` limpio.
4. Antes de publicar, en `rust/`:
   ```powershell
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
   y desde la raíz:
   ```powershell
   .\rust\tools\conformance\gen-golden.ps1
   ```
   que ejecuta **105 escenarios** más la **prueba negativa**.
5. Al terminar `gen-golden.ps1`, restaurar los fixtures (los reescribe el driver):
   ```powershell
   git checkout -- rust/crates/oxidized-imgui-conformance/tests/fixtures/
   ```
6. Si una unidad **no sale verificada, se revierte**. No se fusiona a medias. Esto ha
   pasado varias veces y es correcto: `master` siempre queda limpio y verde.
7. `docs/ESTADO.md` es **generado**: se regenera con
   `pwsh -NoProfile -File .\rust\tools\progress\report.ps1` y CI falla si no coincide
   con `docs/progress.json`. Nunca editarlo a mano.

## Estado al escribir esto

- `master` limpio, **112 PRs** fusionados.
- **105 escenarios** con golden propio, todos en verde, más la prueba negativa.
- **Reescritura: 68.1 %.** Proyecto (contando el arnés): **70.3 %**.
- 32 tests unitarios en 6 binarios.
- La rama concreta está en el último commit de `master`; compruébala con `git log --oneline -1`.

## Método que funciona (y lo que cuesta caro)

Esto está aprendido a base de fallar, y ahorra sesiones enteras:

- **Unidad pequeña, verificada por separado.** Cuando se intentó el combo "del tirón"
  se falló dos sesiones seguidas. Partido en `Selectable` + popup + clic en fila +
  combo, salió a la primera.
- **Funciones NUEVAS antes que reescribir las existentes.** Reescribir funciones largas
  con sustituciones por número de línea rompió `context.rs` tres veces. Añadir funciones
  nuevas junto a las viejas no rompió nada.
- **Medir con una sonda, no adivinar.** Escribir un escenario temporal en
  `driver.cpp` que imprima la geometría de la referencia (rectángulos de ítem,
  tamaños de ventana, orden de ventanas) resuelve en una tirada lo que a ciegas cuesta
  tres intentos. Luego el escenario se convierte en el definitivo y la sonda se borra.
- **Comparar bytes, no líneas.** La comparación por líneas de PowerShell oculta
  diferencias de un carácter (pasó con un nombre de ventana en mayúsculas/minúsculas).

## Trampas conocidas del arnés

- **El fixture es ENTRADA, no salida.** El driver escribe las tablas de glifos y de
  líneas en `tests/fixtures/` y el port las lee. Si un texto usa una letra que no está
  horneada, **la referencia la hornea bajo demanda y el port la pierde**: el dibujo
  cambia en silencio. Hay que hornear **todas** las letras de **todas** las cadenas del
  escenario. Esto ya ha mordido tres veces.
- **El driver lee y escribe `imgui.ini` en el directorio actual**, y una tabla guarda su orden en
  `[Table][...]`: un escenario de tablas que se ejecute donde quedó un ini de otra corrida arranca
  con ese estado. `tables_sort_tristate` es la prueba viva —la referencia en un directorio **limpio**
  devuelve `count 0`, y su golden dice `count 1` porque al generarse el ini ya tenía un `Sort`
  guardado—. Los escenarios nuevos se generan en un **directorio temporal sin `imgui.ini`** y con
  nombre de tabla propio, nunca desde la raíz del repo.
- **El golden se captura con la redirección de `cmd`, no con `>` de PowerShell**: `>` de PowerShell
  convierte los saltos a CRLF y el golden deja de coincidir byte a byte (el `cargo test` lo dice). Por
  eso `gen-golden.ps1` lanza el driver con `cmd /c`.
- Los escenarios con **trazo de flecha** necesitan las filas de línea: usar
  `WriteWindowIconsFixture` en vez de `WriteGlyphFixture`. Cualquier escenario con un **trazo AA**
  (los círculos, por ejemplo) necesita la tabla de líneas entera: `WriteLineUvFixture`, escrita tras
  un frame que haya seleccionado la fuente del atlas. Sin ella el port falla con
  `the atlas line-texture table is not pinned`.
- Las ventanas de tipo **popup, tooltip y menú hijo** se **miden en su primer frame y no
  dibujan nada** en él (la lista vacía no llega a los datos de dibujo). Por eso parecen
  aparecer un frame después.
- Un elemento sólo está "encima" si **su ventana** es la que está bajo el ratón.

## Hecho y verificado (con su golden)

Núcleo y listas: hashes de identificadores, pila de identificadores, ventanas con
posición/tamaño/memoria, cursor y avance de ítems, ratón, tiempo, teclado (incluida la cola de
caracteres y la repetición de una tecla mantenida), fondo, borde,
barra de título, iconos y su interacción, arrastre de ventana, scroll (barra, arrastre,
página, rueda, bloqueo de rueda), auto-ajuste de tamaño, tamaño mínimo, tiradores de
redimensión (bordes, cursor, doble clic, autofit), orden de ventanas, y todas las
primitivas de dibujo y antialiasing, incluidos los círculos (`aa_circles`). Los recortes también
están cerrados: la pila de `PushClipRect`/`PopClipRect` con su intersección (`clip_stack`) y el
corte de comandos de `_OnChangedClipRect` (`cmd_split`: el clip que no cambia no parte, el que
cambia sí, y el push/pop sin geometría se fusiona con el comando anterior).

Widgets: botón (geometría, dibujo, estados, borde), label y checkbox, slider,
**cabecera colapsable**, **nodos de árbol**, **región hija** (lista propia, recorte,
borde en la lista del padre, scroll con rueda y arrastre), **Selectable**, **popup**,
**clic en fila de popup**, **combo**, **tooltip**, **barra de menús**, **submenús
desplegables** (ventana hija, columnas de menú, filas con etiqueta y atajo) y el
**primer corte de las tablas** (`tables_basic`: columnas fijas y filas que crecen con su
contenido; `tables_headers`: la fila de cabecera con su banda y sus nombres, contada por
`TableGetRowIndex`; `tables_rowbg`: las bandas alternas de `RowBg`, que saltan la cabecera;
`tables_cellbg`: el fondo de celda de `TableSetBgColor`, pintado al cerrar la fila;
`tables_borders`: la rejilla interior de `BordersInner`, con la línea fuerte bajo la cabecera;
`tables_borders_outer`: el rectángulo exterior con el atajo alineado de `AddRect`;
`tables_stretch`: las columnas stretch de `SizingStretchSame`, con el resto del reparto al último;
`tables_stretch_weights`: una columna fija y dos stretch con pesos 2 y 1;
`tables_set_column`: celdas direccionadas con `TableSetColumnIndex` en cualquier orden;
`tables_names`: `TableGetColumnCount` y `TableGetColumnName` dentro y fuera de la tabla;
`tables_header_id`: las etiquetas de cabecera cortadas en `##`;
`tables_clip`: el recorte de una celda a su columna y los comandos por columna;
`tables_auto_width`: el ancho automático por contenido, que se ajusta en el segundo frame;
`tables_header_inrow`: el fondo `TableHeaderBg` de una cabecera fuera de su fila;
`tables_column_flags`: los flags derivados de cada columna y sus bits de estado;
`tables_rowbg_headers`: las bandas de `RowBg` cuando la primera fila es de cabecera;
`tables_next_column`: `TableNextColumn` recorriendo la fila y abriendo otra al pasar la última columna;
`tables_header_hover`: el resalte `HeaderHovered` de una cabecera, que llega al tercer frame;
`tables_header_active`: el `HeaderActive` de una cabecera pulsada;
`tables_hovered`: `TableGetHoveredColumn` (inmediato) y `TableGetHoveredRow` (un frame de retraso);
`tables_row_height`: la altura mínima de fila de `TableNextRow`, que gana al contenido;
`tables_sort_arrow`: la flecha de orden que dibuja la cabecera de una tabla ordenable (trazo AA con la
tabla de líneas en el fixture), con ascendente/descendente y el número de orden del segundo criterio;
`tables_sort_click`: el clic de cabecera que alterna la dirección, con el frame del clic (flecha vieja)
y el siguiente (flecha nueva);
`tables_reorder`: el arrastre de una cabecera que reordena las columnas, aplicado en el `BeginTable`
siguiente (dos movimientos, con la columna bajo el ratón como prueba de texto);
`tables_sort_multi`: el Shift+clic que añade una segunda columna al orden (`SortMulti`), con su número
de orden "2";
`tables_scroll_child`: la ventana hija interior de una tabla con `ScrollY` (lista de dibujo propia, dos
listas en el frame, recorte contra el host y el fondo de fila con el clip del host).

La tabla completa, con el nombre del golden de cada fila, está en `rust/README.md`.
El detalle por pieza y su peso, en `docs/progress.json`.

## Pendiente, en el orden sugerido

La entrada de texto (peso 4), las curvas AA (peso 1) y el primer corte de las tablas (peso 1) están
cerradas y su sección queda más abajo como referencia de lo ya medido. Lo que queda:

### Tablas: lo que falta (peso 1)

Están hechos el primer corte (`tables_basic`), las cabeceras (`tables_headers`), las bandas de fila
(`tables_rowbg`), el fondo de celda (`tables_cellbg`), los bordes (`tables_borders` y
`tables_borders_outer`), las columnas stretch (`tables_stretch` y `tables_stretch_weights`), la API
de columnas (`tables_set_column`, `tables_names` y `tables_header_id`), el recorte de celda con sus
comandos por columna (`tables_clip`), las specs de orden (`tables_sort_specs`, `tables_sort_tristate`
y `tables_column_sorted_flag`), la **flecha de orden** (`tables_sort_arrow`, con el número del
segundo criterio y la dirección descendente), el **clic** que alterna la dirección
(`tables_sort_click`), el **reordenado por arrastre** (`tables_reorder`), el **Shift+clic multi-columna**
(`tables_sort_multi`) y la **ventana hija interior de la tabla con scroll** (`tables_scroll_child`). El
resto del sistema, por orden de coste:

- **Políticas de tamaño**: `SizingStretchSame`, los pesos explícitos y el ancho automático por
  contenido ya están; falta el peso de `SizingStretchProp` derivado del contenido (no el declarado).

- **El scroll de la tabla**: la ventana hija interior ya está (`tables_scroll_child`); faltan **su barra**,
  la rueda y el arrastre sobre ella, y con eso **congelar** filas/columnas, porque
  `TableSetupScrollFreeze` sólo cuenta si `InnerWindow->Scroll` no es 0 (`TableSetupScrollFreeze` pone
  `FreezeRowsCount = 0` mientras no haya scroll). Medido: `TableSetupScrollFreeze(0, 1)` en una tabla sin
  scroll deja el volcado **idéntico** al de no usarlo.
- Abierto: con `RowBg` y **dos** columnas, la banda alterna del volcado de referencia sólo cubre
  `68..132` (la primera columna) en vez de `68..220`; hace falta medir de dónde sale ese recorte antes
  de portarlo. Con una columna el golden `tables_rowbg_headers` cuadra.
- Abierto: `TableGetHoveredColumn`/`Row` ya están (`tables_hovered`); hover
  (`tables_header_hover`) y pulsado (`tables_header_active`) también: las cabeceras usan
  `AllowOverlap`, así que sólo reportan hover si ya lo tenían el frame anterior y por eso el resalte
  aparece en el **tercer** frame. Queda el **menú contextual** de la cabecera.

Cifras ya medidas que sirven de contrato (ventana de 400×300 en (40,40), padding 8, fuente 13 px):

- **`CellPadding` por defecto es (4, 2)**, **no** `FramePadding` (que es (4, 3)): corregido con la
  sonda `probe-tables2`. Una columna `WidthFixed` de `w` px mide `w + 2·CellPadding.x` entre `MinX` y
  `MaxX`: 60 px → 68 px de paso, 80 → 88, 100 → 108.
- **Sin bordes, el contenido de la primera columna empieza en `x = 8`**, que es `WindowPadding.x`.
  La segunda columna de 60 px empieza en **76**.
- **Fila de datos: 17 px de alto con texto de 13** (`13 + 2·CellPadding.y` = 13 + 4).
- **El texto de una celda se coloca en el cursor tal cual**: la celda 0,0 pone el texto en
  `(8, 10)` y el cursor de la fila siguiente arranca en `y = 27`.
- **Un ancho explícito en `BeginTable` no cambia nada con columnas fijas**: 3 columnas de 60/80/100
  con `outer_size.x = 300` empiezan en 8, 76 y 164, igual que sin ese ancho.
- Tras `EndTable` el cursor queda por debajo de la última fila: dos filas → `y = 46`; con cabecera,
  tres filas de hueco → `y = 63`.
- **`GetItemRectMin()` es espacio de pantalla**, `GetCursorPos()` es local de la ventana: hay que
  restar la posición de la ventana para compararlos.

## Lo cerrado, con su detalle medido

### Curvas AA (peso 1): círculos

Medido con una sonda **antes** de escribir código (`AddCircleFilled` con `num_segments = 0`, radios
1..40, sobre la lista de una ventana): el número de segmentos sale de la tabla cacheada
`CircleSegmentCounts` (64 entradas, radios 0..63) y el paso de muestreo de
`CalcCornerArcStepAndCount`. La tabla de arco rápido tiene **48 muestras**, no 22
(`IM_DRAWLIST_ARCFAST_TABLE_SIZE`), y el camino de puntos la recorre entera:

| radios | segmentos | paso | puntos del camino | vértices del relleno |
| --- | --- | --- | --- | --- |
| 1 | 4 | 12 | 4 | 8 |
| 2 | 6 | 12 | 4 | 8 |
| 3 | 8 | 6 | 8 | 16 |
| 4–6 | 10 | 4 | 12 | 24 |
| 7–8 | 12 | 4 | 12 | 24 |
| 9–11 | 14 | 3 | 16 | 32 |
| 12–15 | 16 | 3 | 16 | 32 |
| 16–19 | 18 | 2 | 24 | 48 |
| 20–24 | 20 | 2 | 24 | 48 |
| 25–29 | 22 | 2 | 24 | 48 |
| 30–35 | 24 | 2 | 24 | 48 |
| 36–40 | 26 | 1 | 48 | 96 |

El relleno AA emite dos vértices por punto del camino: el anillo interior y uno exterior por
vértice.

Datos fijados en el port, no calculados, porque usan `acos`/`cos` y sus últimos bits no son
portables: la tabla `CircleSegmentCounts[64]` y `ArcFastRadiusCutoff` = `0x430c1e1c` = **140.1176**,
que es hasta donde el círculo se muestrea con la tabla en vez de con trigonometría.

El golden `aa_circles` cubre rellenos y trazos por debajo de ese corte (radios 0.6, 2, 5, 10, 12,
20, 30, 45, 60 y 63), incluido el radio mínimo con su alfa escalada y el trazo más grueso que el
radio, que colapsa en un disco. **No** cubre radios por encima del corte ni `num_segments`
explícito: esos pasan por `_PathArcToN` (`ImCos`/`ImSin`), y hace falta paridad de trigonometría
entre plataformas antes de fijarlos, igual que se hizo con las tablas de arco y de líneas.

### Tablas, primer corte (peso 1)

`Table::update_layout` en `rust/crates/oxidized-imgui/src/tables.rs` resuelve las columnas como
`TableUpdateLayout` para el camino que el golden necesita, y `Context` coloca el cursor de cada celda
y termina la tabla como un ítem de su tamaño final. El golden `tables_basic` pinna el cursor de cada
celda, el rectángulo del texto, los índices de fila y columna y el cursor tras `EndTable`.

Lo que costó, para no repetirlo:

- **`CellPadding` es (4, 2), no (4, 3).** El traspaso anterior decía (4,3) "el mismo `FramePadding`" y
  era falso: la fila mide 17 = 13 + 2·2 y el texto de la primera celda cae en `y = 10`, no en 11.
- **Sin bordes verticales `CellPaddingX` es 0**: los 4 px de cada lado salen de `CellSpacingX1` y
  `CellSpacingX2` (`= CellPadding.x`), no del relleno de la celda. La primera columna empieza 4 px
  antes del contenido (`MinX = WorkRect.Min.x + OuterPaddingX - CellSpacingX1`).
- **Los índices de una tabla no están en orden de dibujo.** `TableBeginCell` cambia el canal del
  `ImDrawListSplitter` a la columna, y `EndTable` los fusiona por orden de columna: los vértices
  quedan como se dibujaron, pero los índices se reagrupan (celdas de la columna 0, luego la 1...).
  Sin ese detalle el golden falla sólo en la sección `idx`.
- **El recorte de la tabla no parte comandos**: `PushClipRect` empuja el `InnerClipRect` de la
  ventana, que es el clip en el que la lista ya está.
- **La banda de una cabecera va al canal de fondo**, no al de la columna: se dibuja al cerrar la fila
  (`TableEndRow`) y el splitter la fusiona **antes** que las columnas. El color por defecto es
  `ImGuiCol_TableHeaderBg` = `0xFF4A4747` (0,28/0,28/0,29), y la fila mide 13 + 2·`CellPadding.y`.
- **El ítem de una cabecera es la celda entera** (`bb` va de `cell_r.Min` a `cell_r.Max`, con la
  altura del texto si es mayor), no la caja del texto; y tras `ItemSize` el cursor se corrige con
  `-ItemSpacing.y * 0.5`.
- **`RowBg` salta las filas de cabecera**: `RowBgColorCounter` sólo sube en las filas que no son de
  cabecera, así que con cabecera + cuatro filas de datos las bandas caen en la segunda y la cuarta,
  y `TableRowBg` es transparente (`0x00000000`), o sea que esas filas no dibujan nada.

### Tablas, flecha de orden (peso 1): cerrada

`TableHeader` reserva el extremo derecho de **toda** columna ordenable y dibuja ahí la flecha, con el
número de orden del segundo criterio. Medido con el golden `tables_sort_arrow` (ventana `S` de 300×200
en (60,60), tabla `SORT` de 3 columnas de 60/80/100, `Sortable | SortMulti`, A ascendente y B
descendente):

- `w_arrow = ImTrunc(FontSize * 0.65 + FramePadding.x)` = `trunc(8.45 + 4)` = **12**.
- `x = max(cell.Min.x, cell.Max.x - w_arrow - w_sort_text)`; el número va en
  `x + ItemInnerSpacing.x` (4) y `w_sort_text = 4 + CalcTextSize("2").x` = **11** con 7 px por avance.
- La flecha va en `x + FontSize * (1 - 0.65) * 0.5` = `x + 2.275`, con `scale = 0.65`.
- **La flecha es un TRAZO, no un relleno**: `SymbolArrowThicknessFactor` vale 0.30, así que
  `r = 13·0.5·0.65 = 4.225`, `thickness = round(clamp(1.2675, 1, 2.1125)) = 1` y se emite como
  `_AddPolyline` de 3 puntos con `StrokeInside | AALineEnds` (**5 quads = 10 vértices por flecha**).
  Por eso el escenario necesita la **tabla de líneas** en el fixture: `WriteWindowIconsFixture` sirve
  (glifos + filas de línea en el mismo fichero).
- Dirección: `ImGuiDir_Up` = `(0, -1)` para ascendente, `ImGuiDir_Down` = `(0, +1)` para descendente.
  El centro es `pos + r + bias·cos/sin`, con `bias = thickness·0.8·0.7071`.
- **El número de orden se pinta al 70 % de alfa y da `0xB3FFFFFF`, no `0xB2`**: el reference pasa por
  `ColorConvertFloat4ToU32`, que hace `saturate(w)·255 + 0.5`; en float32 `0.7f·255` redondea a
  `178.5` y el `+0.5` lo lleva a `179`. En el port está en `Style::color_u32_alpha`.
- **Orden de dibujo**: banda de cabecera (canal de fondo), luego número y flecha, luego la etiqueta.
- `SortMulti` es **`1 << 26`** (no un bit junto a `Sortable`); `PreferSortDescending` es `1 << 15`.
- Con dos columnas `DefaultSort` las dos nacen con `SortOrder = 0`: es `TableSortSpecsSanitize` quien
  las linealiza a 0 y 1, y sólo corre desde `TableSortSpecsBuild`, o sea al llamar a
  **`TableGetSortSpecs`** (que además bloquea el layout antes). Sin esa llamada, las dos dibujan
  flecha sin número.
- El estado de orden (`SortOrder`/`SortDirection`) **sobrevive entre frames** por identificador de
  tabla (`Context::table_sort_state`), igual que los anchos de contenido: los flags `DefaultSort` sólo
  eligen el estado inicial de una tabla que no tiene ninguno. Sin eso, el clic del siguiente corte se
  perdería al re-declarar las columnas.

### Tablas, clic de orden (peso 1): cerrada

El clic de cabecera que cambia las specs, con el golden `tables_sort_click` (12 frames: reposo, entrada
del ratón, pulsación, mantenido, suelte y el frame siguiente, dos veces para asc → desc → asc):

- El clic es **soltar sobre la cabecera que se pulsó** (`ButtonBehavior` con `AllowOverlap`), y se
  aplica **después** de dibujar la flecha: el frame del suelte aún muestra la flecha vieja y el
  siguiente la nueva. Ese desfase es la mitad del golden.
- `TableGetColumnNextSortDirection`: la lista de direcciones disponibles es `[asc, desc]`
  (`PreferSortDescending` las intercambia y `SortTristate` añade "none" al final); una columna sin
  orden toma la primera, la ordenada toma la siguiente de forma cíclica.
- `TableSetColumnSortDirection`: sin `SortMulti` —o sin Shift— las demás columnas pierden su orden y
  esta toma 0; con `append` toma `max + 1`.
- **`SpecsDirty` (el que lee el llamante) no se limpia solo**: se pone al reconstruir las specs y
  sigue a 1 hasta que el llamante lo escribe a false. El flag interno `IsSortSpecsDirty` es el que se
  limpia al entregarlas. Por eso el golden imprime `dirty 1` en los 12 frames: el escenario nunca
  limpia `SpecsDirty`. En el port, `Table::sort_specs_public_dirty` es ese flag y
  `Context::table_clear_sort_specs_dirty` es la escritura del llamante.
- La sonda `probe-sortclick` (en `rust/tools/conformance/out/`) fue lo que lo aclaró: leyendo
  `IsSortSpecsDirty` sale 0 desde el segundo frame, mientras que el `specs->SpecsDirty` que imprime el
  driver sale 1 siempre.
- El estado de orden persiste por id de tabla (`Context::table_sort_state`, ya usado por la flecha);
  sin eso el clic se perdería al re-declarar las columnas en el frame siguiente.
- **Shift+clic** (`tables_sort_multi`): con `SortMulti`, `TableSetColumnSortDirection(..., append =
  IO.KeyShift)` añade la columna en `max(SortOrder) + 1` en vez de sustituir las demás; sin `SortMulti`
  o sin Shift, sustituye. La segunda columna dibuja su número "2" (la misma geometría de la flecha).
- **Trampa que cazó el golden**: `ButtonBehavior` sólo consume el `ActiveId` **la cabecera que lo
  tiene**. El primer intento hacía `header_active.take()` en cualquier cabecera, así que al soltar
  sobre la segunda columna la primera de la fila se comía el release y el clic no se aplicaba. Ahora
  sólo se limpia cuando `header_active` es la clave de esa cabecera, como en el original.

### Tablas, reordenar por arrastre (peso 1): cerrada

Golden `tables_reorder`: 17 frames con dos movimientos (B antes de A y C antes de A) sobre una tabla
`Reorderable` de tres columnas fijas.

- El arrastre: con la cabecera pulsada, `IsMouseDragging(0)` (umbral de 6 px medido desde la pulsación) y
  el ratón cruzando el borde de su celda en el sentido del delta, `TableQueueSetColumnDisplayOrder` guarda
  `ReorderColumn` y `ReorderColumnDstOrder`. **El movimiento no se aplica en ese frame**: lo aplica el
  `BeginTable` siguiente (`TableSetColumnDisplayOrder`), así que el frame del arrastre aún dibuja el orden
  viejo. En el golden se ve en que el frame 4 sigue con el orden A,B,C y el 5 ya trae B,A,C.
- `TableSetColumnDisplayOrder` lleva la columna al orden destino y desplaza ±1 las de en medio;
  `TableGetMaxDisplayOrderAllowed` recorta el destino para no cruzar una columna `NoReorder` (en la
  referencia también las congeladas; aquí no están modeladas).
- `ReorderColumn` se mantiene mientras la cabecera sigue pulsada (`header_active` en el port, `g.ActiveId`
  en la referencia) y se limpia en el `BeginTable` siguiente al suelte; mientras vale `column_n`, el clic
  de orden de **esa** columna se suprime (`pressed && ReorderColumn != column_n`).
- **El layout sigue el orden de display** (la cadena de `MinX`/`MaxX` y `NextEnabledColumn`), pero **los
  canales de dibujo no**: `TableSetupDrawChannels` los asigna por índice de columna y
  `TableMergeDrawChannels` los fusiona en orden de canal, o sea por índice. Los **bordes** sí siguen el
  orden de display. Ese detalle costó una tirada: el primer intento fusionaba por display y el golden
  difería sólo en la sección `idx`.
- La prueba de texto del movimiento es `TableGetHoveredColumn` con el ratón quieto: en x = 170 la columna
  bajo el ratón es la 0 antes del segundo movimiento y la 2 después, porque es la geometría la que se
  mueve, no los índices.
- Geometría: con columnas de 60/80/100 el paso es 68/88/108; tras el primer movimiento el orden B, A, C
  ocupa en pantalla `64..152`, `152..220` y `220..328`.

### Tablas, ventana hija del scroll (peso 1): cerrada

`TableSetupScrollFreeze` no hace nada sin scroll, así que el primer corte del scroll es la **ventana
hija interior** que crean `ScrollY`/`ScrollX`, con golden `tables_scroll_child`:

- `BeginTableEx` crea la hija con `BeginChildEx` y **padding 0, borde 0**: su rectángulo es el de la
  tabla y el contenido arranca en su esquina.
- La hija tiene **lista de dibujo propia**: el `ImDrawData` del frame trae **una sola lista** (la de la
  hija, `S/SCROLL_<id:08X>`), porque la del host queda vacía y `AddDrawListToDrawData` descarta las
  listas sin vértices.
- Su clip es su rectángulo **intersecado con el clip del host** y redondeado: `(68,68)-(352,259)` en el
  golden, porque la tabla mide 400 de alto y la ventana 200, así que se recorta.
- **El fondo de fila no usa ese clip**: `TableEndRow` sobreescribe el clip del comando con
  `Bg0ClipRectForDrawCmd` = `OuterWindow->ClipRect` (el del **host**). Por eso el golden tiene **dos
  comandos**: el fondo (2 quads con clip `(61,61)-(359,259)`) y el resto (clip de la tabla
  `(68,68)-(352,259)`), con todos los vértices en un solo buffer. En una tabla sin scroll los dos clips
  coinciden y el merge los deja en **un** comando; por eso ningún golden anterior lo había visto.
- El layout de la tabla usa `WorkRect`/`InnerRect` de la hija, y `EndTable` cierra la hija (`EndChild`
  avanza el host por el tamaño de la tabla) sin estirar `OuterRect.Max.y` al contenido.
- Medición: con 2 filas el contenido de la hija mide 51 px (17 de cabecera + 2×17). Con 10 filas y
  100 px de alto, `ScrollMax.y = 87` (187 de contenido − 100) y la barra aparece en el segundo frame.

### Entrada de texto (peso 4): cerrada

La pieza grande que quedó cerrada entre los PRs 71 y 76: estado de edición, cursor, selección,
teclado, ratón, portapapeles y scroll interno. **Se midió con una sonda** (`InputText` de una
línea, etiqueta `##field`, `SetNextItemWidth(150)`, ventana de 300×200 en (60,60), fuente 13,
7 px por avance):

**Geometría**

- `frame_size = (150, label_size.y + FramePadding.y * 2)` = `(150, 19)`; rect del ítem
  `(68,68)–(218,87)`; `draw_pos = frame_bb.Min + FramePadding` = `(72, 71)`.
- El marco es **siempre** `FrameBg` (`ff1a1a1a`): no cambia al pasar por encima ni al estar activo
  (`RenderFrame(min, max, FrameBg, true, FrameRounding)`). `FrameRounding` y `FrameBorderSize`
  valen 0 por defecto, así que es un solo relleno.
- El recorte del texto es `(frame_bb.Min, frame_bb.Min + frame_size)` = `(68,68)–(218,87)` y se
  aplica **glifo a glifo** (`CpuFineClip`), no como comando de recorte: la lista sigue con los
  mismos 2 comandos. Hacen falta 17 glifos de 21 letras para 150 px (los espacios no pintan quad).
- Orden dentro del comando de contenido: **marco, selección, texto, cursor**.

**Cursor**

- `cursor_offset.y = (línea + 1) * FontSize` → 13 en una línea.
- `cursor_screen_pos = trunc(draw_pos + cursor_offset - Scroll)`.
- Quad del cursor = `(x, y - FontSize + 1)–(x + 1, y - 1)` = `(x,72)–(x+1,83)`: 1 px de ancho, con
  el **UV del píxel blanco**, o sea un `prim_rect` normal. Se pinta **después** del texto.
- Color `ImGuiCol_InputTextCursor` = blanco (`ffffffff`), el mismo que `Text`: no se distingue por
  color. Visibilidad: `CursorAnim` empieza en −0.20, crece con `DeltaTime`, y el cursor se ve
  mientras `fmod(CursorAnim, 1.20) <= 0.80`.

**Selección**

- Rect por línea: `Min = (draw_pos.x - Scroll.x + CalcTextSize(línea..inicio_selección).x,
  draw_pos.y - 1)`, `Max = (Min.x + ancho, draw_pos.y + 2 + FontSize)` → una letra seleccionada da
  `(72,70)–(79,86)`.
- Color `ImGuiCol_TextSelectedBg` (`59fa9642`), pintado **antes** del texto.

**Scroll horizontal**

- Sólo lo mueve el teclado (`state->CursorFollow`); un clic coloca el cursor **sin** desplazar.
- `scroll_increment = inner_width * 0.25` = 37.5; `visible_width = inner_width - FramePadding.x` = 146.
  Si `offset.x < Scroll` → `Scroll = trunc(max(0, offset.x - increment))`;
  si no, si `offset.x - visible_width >= Scroll` → `Scroll = trunc(offset.x - visible_width + increment)`.
- Medido: cursor al final (offset 189) → `Scroll = 80`, cursor pintado en x = 181.

**Clic** (una línea): `mouse_y = FontSize * 0.5`, y el carácter sale de `mouse_x - draw_pos.x`
(7 px por avance). Clic en x = 100 → desplazamiento 28 → carácter 4 → cursor en x = 100.

**Lo que hay que construir, en este orden** (cada paso con su golden):

1. ~~`Io`: el juego completo de `ImGuiKey` y la cola `InputQueueCharacters`.~~ **Hecho** (PR 71):
   `Key` es ya la lista completa de `ImGuiKey` (teclado; faltan gamepad y botones de ratón como
   teclas), hay `add_input_character`/`add_input_characters_utf8` con la regla del reference
   (un code point por encima de U+FFFF se convierte en U+FFFD), la cola se vacía al final del frame,
   y `IsKeyPressed`/`IsKeyReleased`/la repetición (`KeyRepeatDelay`/`KeyRepeatRate` con
   `CalcTypematicRepeatAmount`) están verificados en el golden `io_text_events`. Ojo: `KeyCtrl` sale
   **sólo** de `ModCtrl`, no de `LeftCtrl`; el golden lo demuestra.
2. ~~`add_text` con `CpuFineClip`~~ **Hecho** (PR 72): `add_text_in` toma el recorte base y el `fine_clip`, descarta los quads fuera sin partir comandos y recorta posición **y UVs** en el borde; `text_clip` lo verifica con cinco recortes distintos.
3. ~~`InputTextState` y el widget, mitad de ratón~~ **Hecho** (PR 73): `InputTextState` con la copia del buffer, cursor, selección, `Scroll`, `CursorAnim` y `CursorFollow`; el campo dibuja marco, texto recortado al marco, cursor y se activa/libera con el ratón (`input_text`, 10 frames). Queda el teclado.
   `Scroll`, `CursorAnim`, `CursorFollow`, y el manejo de teclas de `stb_textedit`
   (`ImStb::STB_TexteditState`).
4. ~~El widget~~ **Hecho** (PR 74): teclado (escribir, flechas, Home/End, Delete/Backspace), selección con Shift y su resalte, y el scroll horizontal que sólo mueve el teclado (`input_text_edit`, 21 frames). ~~Queda el portapapeles y Enter/Escape~~ **Hecho** (PR 75). ~~Queda la selección con el ratón y el deshacer~~ **Hecho** (PR 76): arrastre, doble clic por palabra y la pila de deshacer/rehacer completa. La entrada de texto queda cerrada.

Quedan fuera de este primer corte: multilínea, `Password`, `Hint`, `ReadOnly`, callbacks y
portapapeles.

### Sin empezar en otras áreas

Fuentes y texto (15 %), docking y viewports (0 %), backends y ejemplos (0 %).

## Lo que enseñó el submenú (para no repetirlo)

- **`CalcTextSize("")` no es (0, 0)**: devuelve `(0, tamaño de fuente)`. Es lo que hace que
  la caja de un `MenuItem` de barra mida 17 px aunque pida altura 0. `font.rs` ya lo hacía
  bien; no lo cambies.
- **`ItemSize` corre con el tamaño *pedido*, antes de que `SpanAvailWidth` ensanche la caja.**
  Si mides el avance con el ancho final, una fila de menú reporta todo el ancho del contenido
  como propio y la ventana crece 8 px por frame para siempre.
- **`FramePadding` es `(4, 3)`**, no `(3, 3)`. Un píxel de diferencia desplaza el texto de la
  barra y tira el golden entero.
- La barra de menús no empieza en el padding: empieza en
  `max(WindowPadding.x, ItemSpacing.x)` = 8 desde el borde izquierdo de la ventana.
- **`window_just_activated_by_user`** es "la ventana no se envió en el frame anterior", no
  "es nueva": de eso depende que `MenuColumns` bloquee o tire sus medidas. En el port se
  guarda `WindowMemory.last_frame` para saberlo.
- Un `MenuColumns` que no se bloquea se nota **sólo** en el primer frame dibujado, y ahí la
  ventana aún se está midiendo: por eso el escenario tiene que llegar al segundo frame.

## Cómo comprobar que todo sigue sano

```powershell
cd "D:\Espacio de trabajo\oxidized-imgui"
.\rust\tools\conformance\gen-golden.ps1          # 105 escenarios + negativa
pwsh -NoProfile -File .\rust\tools\progress\report.ps1 -Check   # ESTADO al día
cd rust; cargo test; cargo clippy --all-targets -- -D warnings  # tests y lints
```

Si los tres pasan y `git status` está vacío, el proyecto está sano para empezar.
