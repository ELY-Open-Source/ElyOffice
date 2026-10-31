args <- commandArgs(trailingOnly = TRUE)
hex_read <- function(x) {
  if (x == "-") return("")
  if (!grepl("^([0-9a-f]{2})+$", x)) stop("Invalid UTF-8 transport")
  text <- rawToChar(as.raw(strtoi(substring(x, seq(1, nchar(x), 2),
                                             seq(2, nchar(x), 2)), 16L)))
  Encoding(text) <- "UTF-8"
  text
}
hex_write <- function(x) {
  bytes <- charToRaw(enc2utf8(x))
  if (!length(bytes)) "-" else paste(sprintf("%02x", as.integer(bytes)), collapse = "")
}
lines <- readLines(args[1], encoding = "UTF-8", warn = FALSE)
shape <- as.integer(strsplit(lines[1], "\t", fixed = TRUE)[[1]])
nr <- shape[1]; nc <- shape[2]
decode <- function(kind, text) {
  switch(kind, text = text, number = as.numeric(text), boolean = text == "true",
         empty = NULL, stop("Unknown cell type"))
}
cells <- lapply(lines[-1], function(line) {
  fields <- strsplit(line, "\t", fixed = TRUE)[[1]]
  value <- decode(fields[1], hex_read(fields[2]))
  if (length(fields) == 3L) {
    structure(list(value = value, formula = hex_read(fields[3])), class = "ely_formula")
  } else value
})
original <- structure(lapply(seq_len(nc), function(col) {
  cells[seq(col, nr * nc, by = nc)]
}), names = paste0("V", seq_len(nc)), class = "data.frame", row.names = seq_len(nr))
selected <- structure(list(rows = nr, columns = nc), class = "ely_selection")
operations <- character()
check_range <- function(range) {
  if (!identical(range, selected)) stop("Only the captured selection is available")
}
ely <- list(
  selection = function() selected,
  read_range = function(range) { check_range(range); original },
  write_range = function(range, data) {
    check_range(range)
    if (!is.data.frame(data) || nrow(data) != nr || ncol(data) != nc ||
        !all(vapply(data, is.list, logical(1)))) stop("Keep the list-column data frame shape")
    writes <- character()
    for (row in seq_len(nr)) for (col in seq_len(nc)) {
      old <- original[[col]][[row]]; value <- data[[col]][[row]]
      if (inherits(old, "ely_formula")) {
        if (!identical(old, value)) stop("Formula cells are read-only in this demo")
        next
      }
      if (identical(old, value)) next
      if (is.null(value)) { kind <- "empty"; text <- "" }
      else if (length(value) != 1L || is.na(value) || !is.atomic(value) || is.object(value)) {
        stop("Use NULL, one UTF-8 string, finite number or boolean")
      } else if (is.character(value)) { kind <- "text"; text <- value }
      else if (is.logical(value)) { kind <- "boolean"; text <- if (value) "true" else "false" }
      else if (is.numeric(value) && is.finite(value)) {
        kind <- "number"; text <- sprintf("%.17g", value)
      } else stop("Unsupported cell value")
      writes <- c(writes, paste("W", (row - 1L) * nc + col - 1L, kind, hex_write(text), sep = "\t"))
    }
    operations <<- c(operations, writes)
    invisible(NULL)
  },
  format_header = function(range, bold = TRUE) {
    check_range(range)
    if (!is.logical(bold) || length(bold) != 1L || is.na(bold)) stop("bold must be TRUE or FALSE")
    operations <<- c(operations, paste("H", if (bold) "true" else "false", sep = "\t"))
    invisible(NULL)
  },
  autofit_columns = function(range) {
    check_range(range); operations <<- c(operations, "A"); invisible(NULL)
  }
)
source(args[2], local = new.env(parent = globalenv()), encoding = "UTF-8", echo = FALSE)
writeLines(c("ELY_R_1", operations, "DONE"), args[3], useBytes = TRUE)
