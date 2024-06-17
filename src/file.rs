use std::path::Path;
use tantivy::{schema::Schema, IndexWriter, doc, Term};
use anyhow::Result;
use async_trait::async_trait;
use tokio::fs;
use tokio::task::spawn_blocking;
use futures::future::BoxFuture;
use std::collections::{HashSet, HashMap};
use crate::indexes::Indexable;
use crate::intelligence::{TreeSitterFile, TSLanguage};
use crate::symbol::SymbolLocations;
use crate::schema::build_schema;
use sha2::{Sha256, Digest};
use globset::{Glob, GlobSet, GlobSetBuilder};

pub struct File {
    pub schema: Schema,
    pub path_field: tantivy::schema::Field,
    pub content_field: tantivy::schema::Field,
    pub symbol_locations_field: tantivy::schema::Field,
    pub symbols_field: tantivy::schema::Field,
    pub line_end_indices_field: tantivy::schema::Field,
    pub lang_field: tantivy::schema::Field,
    pub hash_field: tantivy::schema::Field,
    content_insensitive_field: tantivy::schema::Field
}

impl File {
    pub fn new() -> Self {
        let schema = build_schema();
        let path_field = schema.get_field("path").unwrap();
        let content_field = schema.get_field("content").unwrap();
        let symbol_locations_field = schema.get_field("symbol_locations").unwrap();
        let symbols_field = schema.get_field("symbols").unwrap();
        let line_end_indices_field = schema.get_field("line_end_indices").unwrap();
        let lang_field = schema.get_field("lang").unwrap();
        let hash_field = schema.get_field("hash").unwrap();
        let content_insensitive_field = schema.get_field("content_insensitive").unwrap();

        Self {
            schema,
            path_field,
            content_field,
            symbol_locations_field,
            symbols_field,
            line_end_indices_field,
            lang_field,
            hash_field,
            content_insensitive_field
        }
    }

    fn detect_language(path: &Path) -> &'static str {
        let extension = path.extension().and_then(std::ffi::OsStr::to_str).unwrap_or("");
        TSLanguage::from_extension(extension).unwrap_or("plaintext")
    }
}

#[async_trait]
impl Indexable for File {
    async fn index_repository(&self, root_path: &Path, writer: &IndexWriter) -> Result<()> {
        let existing_docs = load_existing_docs(writer, &self.hash_field, &self.path_field)?;
        let mut gitignore = GlobSetBuilder::new();

        traverse_and_index_files(
            root_path, writer, &self.schema, self.path_field, self.content_field,
            self.symbol_locations_field, self.symbols_field, self.line_end_indices_field,
            self.lang_field, self.hash_field, self.content_insensitive_field, 
            &existing_docs, &mut gitignore, root_path).await
    }

    fn schema(&self) -> Schema {
        self.schema.clone()
    }
}

fn load_existing_docs(writer: &IndexWriter, hash_field: &tantivy::schema::Field, path_field: &tantivy::schema::Field) -> Result<HashMap<String, String>> {
    let searcher = writer.index().reader()?.searcher();
    let mut existing_docs = HashMap::new();

    for segment_reader in searcher.segment_readers() {
        let store_reader = segment_reader.get_store_reader(0)?;
        let alive_bitset = segment_reader.alive_bitset();

        for doc in store_reader.iter(alive_bitset) {
            let doc = doc?;
            let path = doc.get_first(*path_field).unwrap().as_text().unwrap().to_string();
            let hash = doc.get_first(*hash_field).unwrap().as_text().unwrap().to_string();
            existing_docs.insert(path, hash);
        }
    }

    Ok(existing_docs)
}

async fn parse_gitignore(current_path: &Path, builder: &mut GlobSetBuilder) -> Result<()> {
    let gitignore_path = current_path.join(".gitignore");

    if gitignore_path.exists() {
        let contents = tokio::fs::read_to_string(&gitignore_path).await?;
        for line in contents.lines() {
            let trimmed_line = line.trim();
            if !trimmed_line.starts_with('#') && !trimmed_line.is_empty() {
                let absolute_pattern = if trimmed_line.starts_with('/') {
                    // The pattern is already an absolute path, so we just use it as is
                    current_path.join(trimmed_line.trim_start_matches('/'))
                } else {
                    // The pattern is a relative path, so we join it with the current path
                    current_path.join(trimmed_line)
                };
                let pattern = absolute_pattern.to_string_lossy().replace("\\", "/");
                // println!("Adding to gitignore: {}", pattern);
                builder.add(Glob::new(&pattern)?);
            }
        }
    }

    Ok(())
}


fn traverse_and_index_files<'a>(
    path: &'a Path,
    writer: &'a IndexWriter,
    schema: &'a Schema,
    path_field: tantivy::schema::Field,
    content_field: tantivy::schema::Field,
    symbol_locations_field: tantivy::schema::Field,
    symbols_field: tantivy::schema::Field,
    line_end_indices_field: tantivy::schema::Field,
    lang_field: tantivy::schema::Field,
    hash_field: tantivy::schema::Field,
    content_insensitive_field: tantivy::schema::Field,  // New field
    existing_docs: &'a HashMap<String, String>,
    gitignore: &'a mut GlobSetBuilder,
    root_path: &'a Path,
) -> BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        // Parse .gitignore in the current directory and update the builder
        parse_gitignore(path, gitignore).await?;

        // Build the GlobSet from the builder
        let globset = gitignore.build()?;

        let mut entries = fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Convert the path to an absolute path
            let absolute_path = path.canonicalize()?;
            let absolute_path_str = absolute_path.to_string_lossy().replace("\\", "/");

            // Skip paths that match .gitignore patterns
            if globset.is_match(&absolute_path_str) {
                continue;
            }

            if path.is_dir() {
                traverse_and_index_files(
                    &path, writer, schema, path_field, content_field, symbol_locations_field,
                    symbols_field, line_end_indices_field, lang_field, hash_field, content_insensitive_field, 
                    existing_docs, gitignore, root_path).await?;
            } else if path.is_file() {
                let path_clone = path.clone();
                let content = spawn_blocking(move || std::fs::read(&path_clone)).await??;

                let content_str = match String::from_utf8(content) {
                    Ok(content_str) => content_str,
                    Err(_) => continue, // Skip if the content is not valid UTF-8
                };

                // Compute the hash of the content
                let mut hasher = Sha256::new();
                hasher.update(&content_str);
                let hash = format!("{:x}", hasher.finalize());

                let path_str = absolute_path_str.clone();
                    if let Some(existing_hash) = existing_docs.get(&path_str) {
                        if existing_hash == &hash {
                            // File has not changed, skip reindexing
                            continue;
                        } else {
                            // Delete the old document
                            writer.delete_term(Term::from_field_text(path_field, &path_str));
                        }
                    }

                let lang_str = File::detect_language(&path);

                if lang_str == "plaintext" {
                    continue;
                }

                let symbol_locations: SymbolLocations = {
                    let scope_graph = TreeSitterFile::try_build(content_str.as_bytes(), lang_str)
                        .and_then(TreeSitterFile::scope_graph);

                    match scope_graph {
                        Ok(graph) => SymbolLocations::TreeSitter(graph),
                        Err(_) => SymbolLocations::Empty,
                    }
                };

                // Flatten the list of symbols into a string with just text
                let symbols = symbol_locations
                    .list()
                    .iter()
                    .map(|sym| content_str[sym.range.start.byte..sym.range.end.byte].to_owned())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join("\n");

                // Collect line end indices as bytes
                let mut line_end_indices = content_str
                    .match_indices('\n')
                    .flat_map(|(i, _)| u32::to_le_bytes(i as u32))
                    .collect::<Vec<_>>();

                // Add the byte index of the last character to the line_end_indices vector
                let last_char_byte_index = content_str.chars().map(|c| c.len_utf8()).sum::<usize>();
                line_end_indices.extend_from_slice(&u32::to_le_bytes(last_char_byte_index as u32));

                // Convert content to lower case for case-insensitive search
                let content_insensitive = content_str.to_lowercase();

                // println!("{}", absolute_path_str);

                let doc = tantivy::doc!(
                    path_field => path_str,
                    content_field => content_str,
                    content_insensitive_field => content_insensitive,  // Add case-insensitive content
                    symbol_locations_field => bincode::serialize(&symbol_locations).unwrap(),
                    symbols_field => symbols,
                    line_end_indices_field => line_end_indices,
                    lang_field => lang_str.to_string(),
                    hash_field => hash,
                );

                writer.add_document(doc)?;
            }
        }
        Ok(())
    })
}
