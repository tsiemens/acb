use crate::portfolio::render::RenderTable;

pub type Error = String;

pub trait AcbWriter {
    fn print_render_table(
        &mut self,
        table_title: &str,
        csv_file_name: &str,
        table_model: &RenderTable,
    ) -> Result<(), Error>;

    fn finish(self: Box<Self>) -> Result<(), Error> {
        Ok(())
    }
}
