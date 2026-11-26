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
   que ejecuta **129 escenarios** más la **prueba negativa**.
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

- `master` limpio, **144 PRs** fusionados (el número sale de la búsqueda de la API: `is:pr is:merged`).
- **129 escenarios** con golden propio, todos en verde, más la prueba negativa.
- **Reescritura: 69.9 %.** Proyecto (contando el arnés): **71.8 %**.
- 42 tests unitarios en 6 binarios (7 de ellos cubren la lógica pura de tablas: linealización de
  órdenes, direcciones disponibles, append con Shift, orden de display y estado persistido).
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
redimensión (bordes, cursor, doble clic, autofit), orden de ventanas, foco por API
(`windows_focus`: `SetNextWindowFocus` sube la ventana al frente y le da la barra activa en el mismo
frame, sin clic) y colapso por API (`windows_collapse_api`), posición por API (`windows_set_pos`) y tamaño por API (`windows_set_size`), y todas las primitivas de dibujo y antialiasing, incluidos los círculos (`aa_circles`). Los recortes también
están cerrados: la pila de `PushClipRect`/`PopClipRect` con su intersección (`clip_stack`) y el
corte de comandos de `_OnChangedClipRect` (`cmd_split`: el clip que no cambia no parte, el que
cambia sí, y el push/pop sin geometría se fusiona con el comando anterior).

Widgets: botón (geometría, dibujo, estados, borde), label y checkbox, slider,
**separador horizontal** (`separator`: ancho de la región de trabajo, grosor `SeparatorSize` y avance
de ítem) y **`SeparatorText`** (`separator_text`: etiqueta entre dos líneas con centro sesgado),
**`BeginDisabled`/`EndDisabled`** (`disabled`), **`Bullet`** (`bullet`) y la **barra de progreso**
(`progress_bar`),
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

### Tamaño de ventana por API (peso 1): cerrada

Golden `windows_set_size`: la misma ventana de `windows_set_pos`, redimensionada con `SetWindowSize`
por nombre.

- `SetWindowSize(name, size)` redimensiona **en el acto** y **trunca** cada eje positivo: (200.9, 120.7)
  deja la ventana en (200, 120), que es lo que muestra el golden.
- El tamaño es el que el siguiente `Begin` lee (memoria de la ventana), y se mantiene en los frames
  siguientes.
- Sólo se modela la forma positiva: en la referencia un eje ≤ 0 no fija el tamaño y enciende el
  auto-ajuste (`AutoFitOnlyGrows`), que no está portado.
- Sin condiciones (`ImGuiCond_*`), igual que `windows_set_pos`.

### Posición de ventana por API (peso 1): cerrada

Golden `windows_set_pos`: una ventana con título y un botón, movida con `SetWindowPos` por nombre.

- `SetWindowPos(name, pos)` mueve la ventana **en el acto** y **trunca** las coordenadas: (120.4, 90.7)
  deja la ventana en (120, 90), que es lo que muestra el golden.
- La posición es la que el siguiente `Begin` lee: no hace falta `SetNextWindowPos`, y la ventana se
  queda ahí en los frames siguientes hasta que algo la mueva otra vez.
- En el port, `Context::set_window_pos` escribe la posición de la memoria de la ventana (`WindowMemory`),
  que es lo que `begin_window` consulta cuando no hay `SetNextWindowPos`. La versión del port para una
  ventana abierta en el mismo frame es la misma, porque la memoria es la fuente.
- No se modelan las condiciones (`ImGuiCond_Once`, `FirstUseEver`, `Appearing`): el escenario usa
  siempre la forma sin condición.

### Separador horizontal (peso 1): cerrada

Golden `separator`: tres etiquetas con un separador entre ellas en una ventana de 300×200 en (60,60).

- El separador va del cursor a `WorkRect.Max.x`: con padding 8 y ancho 300 eso es **x 8..292**, y su
  alto es `max(SeparatorSize, 1)` = **1** (`SeparatorSize` por defecto 1.0).
- Se dibuja como **rectángulo relleno** con `ImGuiCol_Separator` (`0x80806E6E`), no como línea AA: no
  necesita la tabla de líneas en el fixture.
- `ItemSize((0, thickness))`: no aporta ancho al layout y avanza el cursor `thickness + ItemSpacing.y`
  = 5 px. Medido: "A" ocupa `y 8..21`, el separador cae en `y 25..26`, "B" en `30..43`, el segundo
  separador en `47..48`, "C" en `52..65` y el cursor queda en `y 69`.
- El rectángulo del ítem es la línea (`(8,25)-(292,26)`), no la caja del texto: el golden lo imprime
  tras cada `Separator`.
- La forma **vertical** (la barra de menús) tiene su propia sección más abajo.

### InvisibleButton (peso 1): cerrada

Golden `invisible_button`: una ventana con `InvisibleButton("hit", (50, 20))` y un texto, cuatro frames
(reposo, pulsación, suelta, reposo).

- No dibuja nada: la lista de la ventana sólo tiene su fondo. Lo que aporta es el **ítem** (50×20) y
  el `ButtonBehavior`: hover, estado activo y **pulsación al soltar**.
- Medido: `pressed 0` en el frame de la pulsación y `pressed 1` en el de la suelta, como un botón.
- En el port es directamente `interact(id, size)`: la misma rutina que usan el botón y el selectable, así
  que no hay geometría nueva que modelar. Un tamaño con un eje a cero devuelve `false` sin ítem.

### Indent / Unindent (peso 1): cerrada

Golden `indent`: "A", `Indent()`, "B", `Unindent()`, "C".

- `Indent` mueve `DC.Indent.x` y recoloca el cursor en ese borde; `Unindent` lo deshace. El ancho por
  defecto es `Style::IndentSpacing` = **21**.
- En el port ya existían como ayudantes privados del árbol (`indent(width)` / `unindent(width)`); la
  unidad sólo les cambió la firma a `Option<f32>` para que `None` signifique el ancho por defecto y los
  hizo públicos. Siguen moviendo `line_start_x` y el cursor a la vez, que es lo que hace el reference
  con `CursorPos.x = Pos.x + Indent.x`.
- Medido: "A" en `x 68`, "B" en `x 89` (68 + 21) y "C" vuelve a `68`.

### NewLine (peso 1): cerrada

Golden `new_line`: `NewLine()` al principio, "A", `NewLine()` y "B".

- La rama del reference (`if CurrLineSize.y > 0`) **casi nunca** cambia el resultado: cada `ItemSize`
  reinicia `CurrLineSize`, así que después de un texto vale 0 y `NewLine` toma el `else`:
  `ItemSize((0, FontSize))`. Medido: cursor `8 → 25` (línea vacía), "A" en `25..38`, cursor `42`,
  `NewLine` lo deja en `59` (+13+4) y "B" en `59..72` con el cursor final en `76`.
- En el port basta `dummy((0, FontSize))`. La única diferencia posible sería una línea con altura
  fijada a mano (`AlignTextToFramePadding`, la barra de menús), que ningún golden cubre todavía.

### Spacing (peso 1): cerrada

Golden `spacing`: "A", `Spacing()` y "B".

- `Spacing` es un ítem vacío (`ItemSize((0, 0))`): cierra la línea y avanza **sólo** `ItemSpacing.y`.
  Medido: tras "A" el cursor está en `y 25`, tras `Spacing` en `y 29` (+4, no +13+4), porque el
  `ItemSize` anterior ya reinició la altura de línea; "B" ocupa `29..42` y el cursor acaba en `46`.
- En el port basta `dummy((0,0))`: el reinicio de `line_height` al empezar en `line_start_x` ya lo
  modela.

### BulletText (peso 1): cerrada

Golden `bullet_text`: dos entradas en una ventana de 300×200.

- No es "texto + `SameLine` + bullet": es **un solo ítem** de tamaño
  `(FontSize + (etiqueta > 0 ? etiqueta.x + FramePadding.x*2 : 0), etiqueta.y)`. Medido: `(8,8)-(36,21)`
  = 13 + 7 + 8 de ancho y 13 de alto.
- El disco va en `bb.Min + (FramePadding.x + FontSize*0.5, FontSize*0.5)` y el texto en
  `bb.Min + (FontSize + FramePadding.x*2, 0)`, los dos dentro del mismo ítem, sin recorte propio.

### LabelText (peso 1): cerrada

Golden `label_text`: `LabelText("N", "42")` y `LabelText("", "7")` en una ventana de 300×200.

- El ítem se mide con **`total_bb`**: ancho `w + (etiqueta > 0 ? ItemInnerSpacing.x + etiqueta : 0)`
  y alto `max(valor, etiqueta) + FramePadding.y*2`, donde `w` es `CalcItemWidth` (195 = 65 % del
  ancho de la ventana). Medido: con etiqueta `(8,8)-(214,27)` = 195 + 4 + 7; sin etiqueta
  `(8,31)-(203,50)` = 195.
- El valor va en `pos + FramePadding` (3,3) recortado a su caja de ancho `w`; la etiqueta en
  `x = pos + w + ItemInnerSpacing.x` e `y = pos + FramePadding.y`.
- El `SetNextItemWidth` sigue aplicándose: `label_text` consume `calc_item_width` antes de medir.

### Textos con color (peso 1): cerrada

Golden `text_colors`: "A" con `Text`, "B" con `TextColored(rojo)`, "C" con `TextDisabled` y "D" con
`Text`.

- `TextColored` no inventa un camino de dibujo: **empuja `ImGuiCol_Text`** con el color dado,
  dibuja y lo saca, así que el color pasa por `GetColorU32` y `Style::Alpha` sigue aplicándose. En el
  port eso es `Style::color_u32_from([f32; 4])`, que empaqueta `IM_COL32` con `+ 0.5` y multiplica el
  alfa por `Style::Alpha`.
- `TextDisabled` es lo mismo con el color `ImGuiCol_TextDisabled` (`0xFF949494`).
- Los cuatro textos comparten `text_with_colour`, que pinta en el cursor y toma el tamaño en el
  layout: el color no cambia ni la posición ni el avance.

### Barra de progreso (peso 1): cerrada, con un límite anotado

Golden `progress_bar`: cuatro barras en una ventana de 300×200 (25 %, 75 %, una con overlay propio
"half##bar" y otra con el ancho por defecto).

- El tamaño sale de `CalcItemSize(size_arg, CalcItemWidth(), FontSize + FramePadding.y*2)`. El
  **`size_arg` por defecto es `ImVec2(-FLT_MIN, 0)`**, así que la barra **llena el área de contenido**
  (284 px): `CalcItemWidth` (195 = 65 % del ancho) sólo se usa cuando el eje vale **exactamente 0**.
  Medido: la barra por defecto es `(8,77)-(292,96)`.
- Marco: `ImGuiCol_FrameBg` (`0xFF1A1A1A`) como rectángulo plano (`FrameRounding` 0, sin borde).
- Relleno: `ImGuiCol_PlotHistogram`, **pinnado a `0xFF00B3E6`** (r=0xE6, g=0xB3, b=0: la referencia
  empaqueta en orden ABGR).
- Overlay: por defecto `"%.0f%%"` de `fracción*100 + 0.01`, colocado **tras el relleno**
  (`fill_x1 + ItemSpacing.x`), recortado a la barra y centrado en vertical con `align (0, 0.5)`; se
  quita lo que va tras "##" (`FindRenderedTextEnd`), y una barra indeterminada (fracción negativa) no
  lleva texto salvo overlay explícito.
- **Límite anotado**: si el borde del relleno cae en una coordenada fraccionaria, la referencia no usa
  el atajo `PrimRect` sino el camino AA de `AddRectFilled` (8 vértices: un anillo exterior con alfa 0
  y el relleno interior desplazado medio píxel). El port sólo modela el camino alineado a píxel, así
  que el golden usa fracciones que caen en entero (0.25/0.75/0.5 con 200 px y 0.5 con 284) y **la
  barra indeterminada queda pendiente** de ese camino AA (sus bordes casi nunca caen en entero).
- **Trampa del fixture**: el overlay "half" usa letras que el reference hornea al vuelo; el
  `bake_order` tiene que incluir dígitos, `%` y las letras del overlay propio, o el port no dibuja
  esas letras y `calc_text_size` da 0.

### Bullet (peso 1): cerrada

Golden `bullet`: "A", `Bullet()` y "B" en la misma ventana de 300×200.

- El ítem del bullet es **cuadrado con el tamaño de la fuente**: `(8,25)-(21,38)` (13×13), con la
  altura de la línea acotada a `imax(imin(CurrLineSize.y, FontSize + FramePadding.y*2), FontSize)`.
- El disco va en `bb.Min + (FramePadding.x + FontSize*0.5, line_height*0.5)` = (8+4+6.5, 25+6.5) con
  radio `FontSize * 0.2` = 2.6 y **8 segmentos** (`RenderBullet`), que es el camino AA ya portado.
- `SameLine(0, FramePadding.x * 2)` deja el cursor en `x = 21 + 8 = 29`, así que "B" sale sangrada en
  la misma línea y la línea siguiente empieza en `y 42`.

### BeginDisabled / EndDisabled (peso 1): cerrada

Golden `disabled`: "A" y un botón normales, "C" y otro botón dentro de `BeginDisabled`/`EndDisabled`, y
un tercer botón después; cuatro frames (reposo, pulsación y suelta sobre el botón deshabilitado, y hover
sobre el siguiente).

- `BeginDisabled` multiplica **`Style::Alpha`** por `Style::DisabledAlpha` (0.6). Todos los colores
  empaquetados pasan por `GetColorU32`, así que el texto sale con alfa `0x99` en vez de `0xFF` y el
  relleno del botón con su alfa escalada: el efecto es global y no hay que tocar cada widget.
- Los ítems dentro **no hoveran ni pulsan**: `ItemHoverable` devuelve `false` con el flag `Disabled`, y
  en el port `behaviour_at` sale antes de mirar el ratón. Medido: pulsar y soltar sobre el botón
  deshabilitado deja `pressed d 0` y no cambia su relleno (sigue en `ImGuiCol_Button`, sin `Hovered`).
- Anidar sólo cuenta, y `EndDisabled` restaura el alfa guardado por el `BeginDisabled` más externo; el
  botón de después vuelve a alfa `0xFF`.
- El escenario también comprueba que el hover del botón siguiente sigue funcionando (`pressed e 0` pero
  con `ImGuiCol_ButtonHovered`).

### Separador vertical en la barra de menús (peso 1): cerrada

Golden `menu_bar_separator`: `MenuItem("File")`, `Separator()` y `MenuItem("Edit")` dentro de
`BeginMenuBar`, cinco frames.

- Dentro de un layout horizontal `Separator` es **vertical**: `x1 = cursor.x`, `x2 = x1 + grosor`,
  `y1 = cursor.y`, `y2 = cursor.y + CurrLineSize.y`, y **sólo el ancho** entra en el layout
  (`ItemSize((grosor, 0))`), así que la x avanza `grosor + ItemSpacing.x`.
- La altura del ítem no es el texto: la barra hace `AlignTextToFramePadding` al empezar, así que
  `CurrLineSize.y = FontSize + FramePadding.y * 2 = 19`. Medido: el separador va de `y 0` a `y 19`
  (relativo a la ventana) y el cursor queda en `x 61` (52 + 1 + 8).
- **Corrección de modelo**: el port ponía `line_height = 0` al abrir la barra; ningún golden lo
  miraba hasta ahora (los ítems de la barra se colocan con el desfase de línea, no con la altura).
  Ahora `begin_menu_bar` fija esa altura, y `menu_bar`, `menu_submenu`, `combo` y compañía siguen en
  verde.
- **Trampa del fixture**: el botón del cuerpo usa la "x", que el reference hornea al vuelo; el
  `bake_order` del escenario tiene que incluir **todas** las letras de todas las cadenas (las de la
  barra y las de los ítems del cuerpo), o el golden sale con el glifo de más.

### SeparatorText (peso 1): cerrada

Golden `separator_text`: "A", `SeparatorText("S")` y "B" en la misma ventana de 300×200.

- El ítem mide **de ancho** la etiqueta más `SeparatorTextPadding.x * 2` (7 + 40 = 47, pero el
  rectángulo del ítem llega hasta `WorkRect.Max.x`, así que su caja es `(8,25)-(292,44)`), y **de alto**
  `max(label.y + padding.y*2, SeparatorTextBorderSize)` = `13 + 6` = **19**.
- `text_baseline_y = ImTrunc((alto - label.y) * SeparatorTextAlign.y + 0.999)`: con align (0, 0.5) da
  **3**, y la etiqueta se pinta en `pos.x + padding.x` = 28 (la línea de la izquierda acaba en 24, en
  `label.x - ItemSpacing.x`).
- Las líneas van a `trunc((bb.min.y + bb.max.y) * 0.5) + 1` = **35.5 → 35 + 1 = 36** (un `y` entero), con
  grosor `SeparatorTextBorderSize` = 3 y `StrokeCenterBiased`: en píxeles enteros el reference toma el
  atajo `PrimRect(min_x, y - offset)-(max_x, y - offset + 3)` con `offset = (3/2) + (3&1)*(3-3)` = 1.
  Por eso **no** necesita la tabla de líneas (no es un trazo AA en este caso).
- Medición: "A" en `y 8..21`, ítem `25..44`, "B" en `48..61` y el cursor acaba en `y 65`; 20 vértices
  (A, dos líneas, S y B).
- `SeparatorText` recorta la etiqueta a `bb.Max.x` y usa `ItemSpacing.x` a cada lado; sin etiqueta
  dibuja **una sola línea** de extremo a extremo, con su propio golden (`separator_text_empty`):
  el ítem sigue midiendo `max(label + padding, border)` de alto, así que el cursor avanza igual.

### Contratos de color en tests unitarios: cerrada

Tres tests nuevos en `style.rs` fijan lo que antes sólo sujetaban los goldens, para que un cambio de
redondeo no pase desapercibido hasta el CI:

- `color_u32_from` empaqueta en **ABGR** (rojo puro = `0xFF0000FF`) con redondeo `+ 0.5`.
- `Style::Alpha` multiplica el canal alfa en **todos** los colores: `Text` pasa de `0xFFFFFFFF` a
  `0x99FFFFFF` con `DisabledAlpha` = 0.6, tanto por `color_u32` como por `color_u32_from`.
- `color_u32_alpha(Text, 0.70)` da `0xB3FFFFFF` y no `0xB2`: es el redondeo por floats del reference.

Los tests subieron de 39 a **42**.
### Cursor por eje (peso 1): cerrada

Golden `cursor_pos_xy`: `SetCursorPosX(10)` y luego `SetCursorPosY(30)` en una ventana de 300x200.
Medido: el cursor va de `(8,8)` a `(10,8)` y despues a `(10,30)`: cada llamada mueve **solo** su eje y
deja el otro como estaba.

El port los expone como `set_cursor_pos_x` y `set_cursor_pos_y`, que escriben `pos.x + local` y
`pos.y + local` respectivamente, igual que el `SetCursorPos` completo.
### Alturas de item (peso 1): cerrada

Golden `item_heights`: las cuatro consultas de altura del reference, con los bits crudos del float. Medido:
`GetTextLineHeight` = **13**, `GetTextLineHeightWithSpacing` = **17**, `GetFrameHeight` = **19** y
`GetFrameHeightWithSpacing` = **23**, es decir font mas `ItemSpacing.y` (4) y font mas `FramePadding.y * 2`
(6) respectivamente.

El port los expone como `text_line_height`, `text_line_height_with_spacing`, `get_frame_height` y
`frame_height_with_spacing`; los dos ultimos reutilizan el `frame_height()` interno que ya usaban los
widgets enmarcados.
### Fondo de ventana translúcido (peso 1): cerrada

Golden `window_bg_alpha`: la misma ventana con y sin `SetNextWindowBgAlpha(0.5)`. Medido: el primer
vértice de la lista pasa de `0xFF262626` a `0x80262626` (0.5 por 255 mas 0.5 = 128 = 0x80), y el
resto de la ventana no cambia.

El port lo modela con `next_bg_alpha`: al pintar el fondo toma `color_u32_alpha(WindowBg, alpha)` en
vez de `color_u32(WindowBg)` y consume la petición, igual que el resto de `SetNext*`.
### Cursor en coordenadas de ventana (peso 1): cerrada

Golden `cursor_pos`: "A", `SetCursorPos((10, 5))` y "B" en una ventana de 300x200.

- `GetCursorPos` devuelve el cursor **relativo a la ventana** (resta `Pos`), no la posicion de
  pantalla de `GetCursorScreenPos`. Medido: `(8,8)` al empezar, `(8,25)` tras "A", `(10,5)` tras el
  `SetCursorPos` y `(8,22)` tras "B".
- `SetCursorPos` mueve **solo el cursor**, no el borde de la linea: por eso despues de "B" (que se
  dibuja en (10,5) y mide 13 de alto) el cursor vuelve a `x 8` y baja a `y 5+13+4 = 22`. El port lo
  modela sin tocar el borde de la linea, y el golden lo fija.
### Foco de ventana por nombre (peso 1): cerrada

Golden `windows_set_focus`: las mismas dos ventanas solapadas de `windows_focus`, ahora con
`SetWindowFocus` sobre "A" o "B" antes de los `Begin`.

- Es la versión inmediata de lo que ya existía: mueve el nombre al final del orden de display y lo
  deja como ventana enfocada, que es justo el estado que el `Begin` siguiente encuentra hecho. Un
  nombre desconocido no hace nada, como el `FindWindowByName` fallido de la referencia.
- Medido: `[A,B]` → `focus_a` → `[B,A]` (y la barra activa pasa a A) → `focus_b` → `[A,B]`.
### Colapso de ventana por nombre (peso 1): cerrada

Golden `windows_set_collapsed`: la misma ventana de `windows_collapse_api`, ahora con
`SetWindowCollapsed("C", ...)` en vez de `SetNextWindowCollapsed`.

- La diferencia es de **almacenamiento**: `SetNextWindowCollapsed` deja una petición para el próximo
  `Begin`, y `SetWindowCollapsed` escribe directamente el estado recordado de la ventana. El port ya
  lee esa memoria al empezar, así que el método es un `memory.collapsed = collapsed`.
- Medido, igual que la variante de "siguiente ventana": 38 vértices expandida, 26 colapsada (barra +
  flecha), y vuelve a 38; los ítems del cuerpo se siguen saltando.

### Colapso de ventana por API (peso 1): cerrada

Golden `windows_collapse_api`: una ventana con título y un botón en el cuerpo, cinco frames que la
colapsan y la expanden con `SetNextWindowCollapsed`.

- `SetNextWindowCollapsed` deja `g.NextWindowData.CollapsedVal` y lo aplica el **mismo** `Begin`, no el
  siguiente: el frame 1 ya se dibuja colapsado.
- Colapsada, la ventana dibuja **sólo la barra de título** (con su flecha) y se salta los ítems: el
  botón no se dibuja ni reporta pulsación (`SkipItems`). En el port, `button` ahora sale antes de
  calcular nada si la ventana está colapsada.
- La flecha de colapso es un **trazo AA** y muestrea la tabla de líneas: el fixture del escenario es
  `WriteWindowIconsFixture` (glifos **y** filas de línea), no `WriteGlyphFixture`. Sin eso el port no
  puede dibujar la flecha (es la trampa ya documentada arriba).
- Medición: expandida 38 vértices; colapsada 26 (la barra más la flecha, sin fondo del cuerpo).

### Foco de ventana por API (peso 1): cerrada

Golden `windows_focus`: dos ventanas solapadas (A en (60,60) y B en (110,100) de 300×200) y siete
frames que alternan `SetNextWindowFocus` entre ellas.

- `SetNextWindowFocus` sólo deja `g.NextWindowData.Focus`; lo aplica el `Begin` de **esa** ventana como
  `FocusWindow`: la sube al final del orden de display (la primera de `g.Windows` es la de delante) y le
  da el foco. El `ImDrawData` cambia de orden **en el mismo frame** — `[A,B]` → `[B,A]` — y la barra de
  título de la enfocada pasa a `TitleBgActive`; la otra vuelve a `TitleBg`.
- No hay clic, así que no hay arrastre ni `ActiveId`: sólo cambian el orden y el color.
- El port ya modelaba el orden con `Context::display_order` (de atrás a delante) y el foco con
  `focused_window`; la unidad añadió el flag `next_focus`, su `set_next_window_focus()` y el bloque que
  en `begin_window` mueve la ventana al final del orden y la enfoca.
- Medición: el orden de las listas del volcado es la prueba (`list 0`/`list 1` con `A`/`B`), y los
  colores de la barra de título van dentro de esas listas.

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

### Tablas, scroll: la barra, medido y pendiente (peso 1)

Sonda `tables_scroll_bar` (tabla `ScrollY | RowBg`, 3 columnas fijas, 10 filas, `outer_size.y = 100`,
2 frames; el escenario definitivo no está hecho todavía). La barra aparece en el **segundo** frame y
cambia el reparto de comandos de la lista de la hija:

- **Frame 0** (sin barra): `cmd 0` con el clip del **host** (18 elems: los fondos de fila) y `cmd 1`
  con el clip de la tabla `(68,68)-(352,259)` (78 elems: etiquetas, textos y los huecos de las celdas).
- **Frame 1** (con barra): el `InnerClipRect` de la tabla pasa a `(68,68)-(338,259)` (352 − 14 de
  barra) y la lista de la hija trae **tres** comandos: `cmd 0` clip del host con **144 elems** (la
  barra **y** los fondos de fila, fusionados por el splitter porque comparten clip), `cmd 1` clip del
  host con **18 elems** (tres quads, aún por identificar) y `cmd 2` clip de la tabla con 78 elems (el
  contenido).

Lo que eso fija para el port:

- **La barra se dibuja en la lista de la hija con el clip del host**, o sea *antes* del push del
  `InnerClipRect` de la tabla; el port hoy empuja el clip de la hija nada más abrirla, así que la barra
  tiene que ir antes de ese push (o sobre un clip equivalente) para caer en el mismo comando que los
  fondos de fila.
- `draw_scrollbar` sirve, pero hay que **parametrizar el padding**: su `contents` suma
  `style.window_padding * 2` y la ventana hija de la tabla tiene padding 0. También hay que pasarle el
  `outer`/`inner_rect` de la hija (con el `inner_rect` ya encogido por la barra).
- El `InnerClipRect` de la tabla (y con él el clip de las columnas y de sus celdas) se encoge el ancho
  de la barra: `(68,68)-(338,259)`.
- Queda por identificar el `cmd 1` de tres quads antes de poder cerrar la unidad; después, la rueda y
  el arrastre de la barra, y con eso `TableSetupScrollFreeze` (que sólo cuenta si
  `InnerWindow->Scroll` no es 0).
- **Segunda medición, sin `RowBg`**: la estructura no se simplifica. Frame 1: `cmd 0` (144 elems, clip
  del host: la barra), `cmd 1` (6 elems, clip del host: **una** banda de cabecera) y `cmd 2` (78 elems,
  clip de la tabla). O sea que el splitter deja la banda de la cabecera en un comando **propio** aunque
  comparta clip con la barra: no fusiona comandos de distinto *momento* dentro del mismo canal. El
  merge simplificado del port (fondo + columnas) no modela esas fronteras, así que la barra necesita
  primero un modelo de los comandos por canal del splitter (o el golden no se puede igualar). Eso es lo
  que bloquea hoy esta unidad, no la barra en sí.

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
.\rust\tools\conformance\gen-golden.ps1          # 129 escenarios + negativa
pwsh -NoProfile -File .\rust\tools\progress\report.ps1 -Check   # ESTADO al día
cd rust; cargo test; cargo clippy --all-targets -- -D warnings  # tests y lints
```

Si los tres pasan y `git status` está vacío, el proyecto está sano para empezar.
