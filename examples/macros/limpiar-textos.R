seleccion <- ely$selection()
datos <- ely$read_range(seleccion)

# Las columnas son listas: conservan tipos distintos por celda.
datos[] <- lapply(datos, function(columna) {
  lapply(columna, function(celda) {
    if (is.character(celda)) trimws(celda) else celda
  })
})

ely$write_range(seleccion, datos)
ely$format_header(seleccion, bold = TRUE)
ely$autofit_columns(seleccion)
message("Textos limpios; fórmulas conservadas.")
