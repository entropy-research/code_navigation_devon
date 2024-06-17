use tantivy::schema::{Schema, TEXT, STRING, STORED, FAST, BytesOptions, SchemaBuilder};

pub fn build_schema() -> Schema {
    let mut schema_builder = SchemaBuilder::default();
    schema_builder.add_text_field("path", STRING | FAST | STORED);
    schema_builder.add_text_field("content", TEXT | STORED);
    schema_builder.add_text_field("content_insensitive", TEXT | STORED);
    schema_builder.add_bytes_field("symbol_locations", STORED);
    schema_builder.add_bytes_field("line_end_indices", BytesOptions::default().set_stored());
    schema_builder.add_text_field("symbols", TEXT | STORED);
    schema_builder.add_text_field("lang", STRING | FAST | STORED);
    schema_builder.add_text_field("hash", STRING | FAST | STORED);
    schema_builder.build()
}
