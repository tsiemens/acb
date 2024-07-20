pub struct SheetParseError {
    row: usize,
    msg: String,
}

impl SheetParseError {
    pub fn new(row: usize, msg: String) -> Self {
        SheetParseError { row: row, msg: msg }
    }
}

impl std::fmt::Display for SheetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Row {}: {}", self.row, self.msg)
    }
}
