selection <- ely$selection()
data <- ely$read_range(selection)

# Columns are lists: each cell keeps its own type.
data[] <- lapply(data, function(column) {
  lapply(column, function(cell) {
    if (is.character(cell)) trimws(cell) else cell
  })
})

ely$write_range(selection, data)
ely$format_header(selection, bold = TRUE)
ely$autofit_columns(selection)
message("Text trimmed; formulas kept.")
