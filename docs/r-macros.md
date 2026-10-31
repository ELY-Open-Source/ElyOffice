# Macros R experimentales en ElyTable

Esta demo ejecuta R localmente sobre una copia del rango seleccionado. Requiere
R con `Rscript` en PATH, sin paquetes adicionales. ElyOffice no instala R.
R se ejecuta con los permisos del usuario: el proceso separado **no es una
sandbox**. Ejecuta únicamente código de confianza.

## Preparación del desarrollador

Compila el núcleo LibreOffice con la serie de parches del repositorio, incluido
`0063-lok-calc-r-macro-batches.patch`, y después el worker y ElyTable:

```sh
scripts/build-libreoffice.sh
cargo build -p ely-lok --bins -p ely-table
cargo run -p ely-table
```

Un núcleo anterior no ofrece la lectura tipada ni la aplicación del lote. No
basta con recompilar únicamente ElyTable. Sigue README.md para preparar las
herramientas de compilación; la compilación del núcleo queda fuera del minuto
de prueba.

## Prueba en un minuto

1. Puedes abrir `tests/fixtures/table/r-macro-demo.fods` y seleccionar `A1:E3`.
   Para crear otra muestra: en un libro nuevo, introduce una cabecera y textos con espacios al principio
   y al final. Incluye `  José  `, comillas y un identificador de texto `'00123`.
   Añade un número, un booleano, una celda vacía y una fórmula `=1+1`.
2. Selecciona el rectángulo que contiene esos datos, incluida la cabecera.
3. Abre **Macros → Ejecutar macro R — experimental** y pulsa **Ejecutar**.
   La macro de ejemplo ya está en el editor. También puedes abrir
   [limpiar-textos.R](../examples/macros/limpiar-textos.R) con **Abrir .R…**.
4. Comprueba los textos sin espacios, la cabecera en negrita y los anchos
   ajustados. `00123` sigue siendo texto y `=1+1` sigue siendo una fórmula.
5. Cierra el editor y usa **Deshacer** una vez: valores, negrita y anchos vuelven
   juntos al estado previo. **Guardar .R…** guarda solo el código, fuera del libro.

Para probar un fallo, añade `stop("falló")` al final de la macro: el editor
muestra el error y el documento no cambia. `Sys.sleep(60)` provoca el límite
de ejecución de 30 segundos, sin aplicar instrucciones. Si otra operación
modifica el libro mientras R trabaja, el lote se rechaza por conflicto; vuelve
a ejecutarlo sobre la nueva selección.

## API disponible

```r
seleccion <- ely$selection()
datos <- ely$read_range(seleccion)
ely$write_range(seleccion, datos)
ely$format_header(seleccion, bold = TRUE)
ely$autofit_columns(seleccion)
```

La selección es un identificador opaco con `rows` y `columns`. Solo representa
el rectángulo capturado al pulsar Ejecutar. No acepta direcciones arbitrarias.
`read_range` devuelve un `data.frame` cuyas columnas son listas: cada celda
conserva su tipo, incluso en columnas mixtas. Los textos son cadenas UTF-8,
los números son doubles, los booleanos son logical y las vacías son `NULL`.
Una cadena vacía sigue siendo texto al leerla; Calc puede normalizar una
escritura de cadena vacía como celda vacía.

Las fórmulas se representan mediante objetos `ely_formula` con miembros
`formula` (expresión) y `value` (resultado calculado). Son de solo lectura en
esta demo. La macro de ejemplo transforma solo cadenas, dejando intactos
estos objetos. Para cambiar una celda usa, por ejemplo,
`datos[[1]][2] <- list("00123")`; para vaciarla, `datos[[1]][2] <- list(NULL)`.
Conserva las dimensiones y las columnas como listas. Envía como máximo una
escritura de cada celda, una instrucción de cabecera y una de ajuste de ancho.

## Ejecución y límites

ElyTable toma la revisión del documento y una copia tipada del rango mediante
LOK. Libera el worker antes de iniciar `Rscript --vanilla`, con argumentos
separados y archivos temporales privados. El código se carga desde un archivo;
no se construyen comandos de shell con código ni datos del usuario.

La API R lee la copia y acumula instrucciones en memoria. Solo escribe el
resultado cuando el script termina correctamente. ElyTable valida el protocolo
completo, los tipos, los límites, las posiciones y la conservación de fórmulas
antes de enviar una única operación al worker. El worker comprueba la revisión;
el núcleo vuelve a contrastar la selección y sus datos, valida todo el lote y
lo aplica dentro de un contexto de deshacer. Si falla la aplicación, revierte
ese contexto. Un fallo de R o de validación no envía cambios al documento.

Límites: 10 000 celdas, 30 segundos, 4 MiB por archivo de entrada/salida y
32 767 bytes por texto. Se rechazan selecciones múltiples, hojas protegidas y
rangos combinados y documentos sin deshacer habilitado. El ajuste cambia el ancho de las columnas completas
seleccionadas. El lote cambia negrita y ancho cuando se solicitan; al escribir
booleanos usa el formato lógico. Una escritura numérica sobre una celda lógica
vuelve a formato numérico estándar. Las alturas automáticas siguen el recálculo
de Calc, también al deshacer textos.
No se garantiza recuperación ante un cierre forzado o fallo del núcleo a mitad
de una operación; el rollback cubre errores de aplicación ordinarios.

Las macros no se guardan en XLSX/ODS ni se ejecutan al abrir documentos. Esta
API no incluye paquetes, eventos, funciones en celdas, VBA ni servicios remotos.
Los procesos que una macro lance por su cuenta tienen los permisos del usuario;
el límite controla el proceso Rscript que inicia ElyTable.

## Verificación

Las pruebas están en `src/macros/{protocol,runner,integration}.rs` de ElyTable y
`crates/ely-lok/src/client/tests/r_macros.rs`. Con R y el núcleo preparados:

```sh
cargo build -p ely-lok --bins
cargo test -p ely-table macros:: -- --nocapture
ELY_REQUIRE_ENGINES=1 cargo test -p ely-lok r_macros:: -- --nocapture
```

La prueba de R real informa `SKIP` si Rscript falta. La de LOK informa `SKIP`
si falta el núcleo, salvo con `ELY_REQUIRE_ENGINES=1`. Las pruebas de protocolo
y proceso sustituto no requieren R ni LibreOffice.

### Comprobaciones realizadas en esta implementación

- Compilación de ElyTable y el worker; check y Clippy de ambos paquetes y sus
  pruebas, con `-D warnings`: pasan.
- **14 pruebas pasan**: nueve de protocolo/proceso, cuatro nativas LOK y una
  integrada que ejecuta el ejemplo con R real sobre el documento. Incluyen
  UTF-8, comillas, espacios, vacías, ceros iniciales, booleanos, precisión
  numérica, fórmulas, validación completa, errores, ausencia de Rscript y timeout.
  Verifican conflictos, rechazo con deshacer deshabilitado y restauración de
  valores, negrita y anchos con un solo Deshacer. Se ejecutaron importando los
  módulos fuente reales en un harness Cargo aislado con serde_json, tempfile y
  ely-lok del repositorio.
- Calc se compiló y enlazó desde LibreOffice 26.2.6.3, commit
  `8221e31b3ac356a1623c672912a3d2b492f7e3d1`, con los parches 0001–0063.
  El runtime local usa ese Calc y los demás componentes instalados de la misma
  versión fijada. La compilación completa de toda la suite no se repitió.
- Pruebas C++ verifican el transporte exacto de doubles, incluidos extremos,
  cero negativo y subnormales, y la equivalencia de objetos JSON con distinto
  orden de claves, conservando el orden y contenido de las celdas.
- En una ventana real en español se verificaron el menú y el editor. Abrir y
  guardar `.R` mediante el diálogo del sistema conservó el ejemplo byte por
  byte. El usuario completó la prueba desde la GUI: confirmó la ejecución,
  los textos, la cabecera en negrita y los anchos ajustados; la barra de
  fórmulas de E2 mostró `=C2*2` y confirmó que Deshacer funciona.
- El parche se comprueba en reversa contra el árbol compilado. Las reglas de
  parches, `scripts/check.sh`, el formato de los módulos nuevos y
  `git diff --check` pasan. El formato global encuentra diferencias previas en
  archivos ajenos a esta tarea.

R 4.6.1 se instaló por petición explícita del usuario. La demo local se abre
con la entrada **ElyTable R — experimental**, o con
`./.work/r-demo/ely-table-r` desde este checkout. Usa un perfil aislado y abre
una copia de la muestra. El producto sigue sin instalar R automáticamente.

No se verificaron Windows/macOS ni la suite completa del workspace. Se utilizó
Rust 1.98.0 disponible; el repositorio fija 1.98.1. `scripts/review.sh` requiere
herdr y no se pudo ejecutar aquí. Las tres revisiones independientes están en
`tasks/reviews/TABLE-R-r1.md`, `TABLE-R-r2.md` y `TABLE-R-r3.md`. La corrección
posterior del identificador de vista del grupo de deshacer pasó las pruebas
nativas; no recibió otra revisión independiente porque se agotaron las tres
rondas permitidas por el repositorio.

## Archivos modificados

| Archivos | Cambio |
| --- | --- |
| `apps/ely-table/src/macros/*` | Editor, diálogos `.R`, protocolo tipado, API base R, proceso y pruebas. |
| `apps/ely-table/{Cargo.toml,src/book.rs,src/dialog.rs,src/main.rs}` | Integración del menú, acción y editor. |
| `crates/ely-lok/src/commands*` y `client/tests/r_macros.rs` | Registro del puente, control de revisión y pruebas nativas. |
| `patches/libreoffice/0063-lok-calc-r-macro-batches.patch` | Lectura de selección, validación y aplicación con deshacer vinculado a la vista. |
| `crates/ely-ui/src/i18n/{table.rs,es/table.rs}` | Textos en inglés, chino y español. |
| `examples/macros/limpiar-textos.R`, `tests/fixtures/table/r-*` y `rscript-*` | Ejemplo, libro de muestra y procesos sustitutos de prueba. |
| `README.md`, `agents.md`, `PRD.md`, `docs/decisions.md`, `tasks/*` | Uso, arquitectura, alcance, estado y revisiones. |
