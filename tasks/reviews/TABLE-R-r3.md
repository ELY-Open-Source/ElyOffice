PASS

Tercera revisión independiente, 2026-10-31. Alcance: canonicalización de ptree en el parche 0063 y nueva prueba apps/ely-table/src/macros/integration.rs.

La canonicalización recorre los hijos antes de ordenar los objetos; conserva el orden de los arrays porque sus claves son vacías. Se aplica tanto al lote recibido como al snapshot actual. Para las claves no vacías y únicas que genera este protocolo, esto elimina el conflicto falso provocado por el distinto orden de propiedades de serde_json y JsonWriter, sin equiparar celdas reordenadas ni valores distintos. Los números continúan como cadenas decimales, conservando la corrección revisada en la segunda ronda.

La prueba de integración está habilitada con cfg(test), ejecuta la macro de ejemplo mediante R real y el puente del worker, comprueba un error de R sin modificación, textos e identificadores, fórmulas preservadas, cambios de ancho y negrita exportada a FODS. Tras un único Undo exige recuperar el snapshot y los anchos originales y eliminar la negrita añadida. No encontré defectos concretos en los cambios revisados.

Limitaciones: revisión de código sin compilar ni ejecutar pruebas, según el alcance solicitado. La prueba C++ aislada de equivalencia de objetos y diferencia de arrays/textos fue comunicada por el implementador. La compilación nativa de Calc aún está en curso; esta revisión no acredita que la nueva prueba de integración haya pasado. scripts/review.sh requiere herdr y no fue ejecutado; este documento registra la revisión independiente alternativa.

Nota posterior del implementador: las pruebas nativas detectaron que el contexto UNO creaba una lista con ViewShellId(-1), que Calc/LOK rechazaba al deshacer desde la vista activa. Se sustituyó por EnterListAction/LeaveListAction del SfxUndoManager con el identificador de esa vista. Las 14 pruebas pasan, incluida la macro R real y su Deshacer conjunto. Esta corrección posterior no forma parte del PASS de lectura anterior; no se abrió una cuarta ronda.
