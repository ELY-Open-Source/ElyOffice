FAIL

Revisión independiente de código, 2026-10-31. Dos defectos bloqueantes:

1. El snapshot usaba JsonWriter.put(double), cuyo camino OString::number redondeaba a 15 cifras significativas. Una macro podía recibir un número distinto del original.
2. El contraste exacto de árboles JSON comparaba los lexemas numéricos: LibreOffice emitía, por ejemplo, 1E-007, y serde_json podía reenviarlo como 1e-7. Un documento intacto podía rechazarse por conflicto.

Corrección aplicada antes de r2: transportar números como cadenas decimales tipadas, emitirlas con std::to_chars y max_digits10, conservarlas en Rust y validar doubles finitos. Se añadieron regresiones de precisión, exponentes y estabilidad del snapshot. Esta revisión fue de código; no acreditó ejecución del núcleo nuevo.
