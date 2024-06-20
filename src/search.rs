use std::collections::HashMap;
use std::path::Path;
use tantivy::query::{FuzzyTermQuery, TermQuery, QueryParser};
use tantivy::schema::Field;
use tantivy::{Index, IndexReader, collector::TopDocs, Term};
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::content_document::ContentDocument;
use crate::intelligence::code_navigation::{CodeNavigationContext, FileSymbols, OccurrenceKind, Token};
use crate::intelligence::TSLanguage;
use crate::schema::build_schema;
use crate::symbol::SymbolLocations;
use crate::text_range::TextRange;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub line_number: usize,
    pub column: usize,
    pub context: String,
}

pub struct Searcher {
    index: Index,
    reader: IndexReader,
    path_field: Field,
    content_field: Field,
    content_insensitive_field: Field, // Added field
    line_end_indices_field: Field,
    lang_field: Field, // Added lang field
    symbol_locations_field: Field,
}

impl Searcher {
    pub fn new(index_path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_path)?;
        let reader = index.reader()?;
        let schema = build_schema();
        let path_field = schema.get_field("path").unwrap();
        let content_field = schema.get_field("content").unwrap();
        let content_insensitive_field = schema.get_field("content_insensitive").unwrap(); // Added field
        let line_end_indices_field = schema.get_field("line_end_indices").unwrap();
        let lang_field = schema.get_field("lang").unwrap();
        let symbol_locations_field = schema.get_field("symbol_locations").unwrap();

        Ok(Self {
            index,
            reader,
            path_field,
            content_field,
            content_insensitive_field,
            line_end_indices_field,
            lang_field,
            symbol_locations_field,
        })
    }
    
    pub fn text_search(&self, query_str: &str, case_sensitive: bool) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        
        // Choose the appropriate field and query parser based on case sensitivity
        let (field, query_str) = if case_sensitive {
            (self.content_field, query_str.to_string())
        } else {
            (self.content_insensitive_field, query_str.to_lowercase())
        };
    
        let query_parser = QueryParser::for_index(&self.index, vec![field]);
        let query = query_parser.parse_query(&query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
    
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
    
            let path = match retrieved_doc.get_first(self.path_field) {
                Some(path_field) => path_field.as_text().unwrap().to_string(),
                None => {
                    println!("Debug: Path field is missing");
                    continue;
                }
            };
    
            let content = match retrieved_doc.get_first(field) {
                Some(field) => field.as_text().unwrap().to_string(),
                None => {
                    println!("Debug: Content field is missing");
                    continue;
                }
            };

            let new_content = match retrieved_doc.get_first(self.content_field) {
                Some(content_field) => content_field.as_text().unwrap().to_string(),
                None => {
                    println!("Debug: Content field is missing");
                    continue;
                }
            };
    
            let line_end_indices_field = retrieved_doc.get_first(self.line_end_indices_field);
    
            let line_end_indices: Vec<u32> = match line_end_indices_field {
                Some(field) => {
                    match field.as_bytes() {
                        Some(bytes) => {
                            bytes.chunks_exact(4).map(|c| {
                                u32::from_le_bytes([c[0], c[1], c[2], c[3]])
                            }).collect()
                        }
                        None => {
                            println!("Debug: Failed to get bytes");
                            continue;
                        }
                    }
                }
                None => {
                    println!("Debug: Line end indices field is missing");
                    continue;
                }
            };
    
            for (mut line_number, window) in line_end_indices.windows(2).enumerate() {
                if let [start, end] = *window {
                    let line = &content[start as usize..end as usize];
    
                    if line.contains(&query_str) {
                        line_number += 2;
                        let column = line.find(&query_str).unwrap();
                        let context_start = if line_number >= 3 { line_number - 3 } else { 0 };
                        let context_end = usize::min(line_number + 3, line_end_indices.len() - 1);
                        let context: String = line_end_indices[context_start..=context_end]
                            .windows(2)
                            .map(|w| {
                                let start = w[0] as usize;
                                let end = w[1] as usize;
                                &new_content[start..end]
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
    
                        results.push(SearchResult {
                            path: path.clone(),
                            line_number,
                            column,
                            context,
                        });
                    }
                }
            }
        }
    
        Ok(results)
    }
    

    pub fn fuzzy_search(&self, query_str: &str, max_distance: u8) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        
        let query = FuzzyTermQuery::new(
            Term::from_field_text(self.content_field, query_str),
            max_distance,  // max edit distance for fuzzy search
            true,
        );
    
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
    
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
    
            let path = match retrieved_doc.get_first(self.path_field) {
                Some(path_field) => path_field.as_text().unwrap().to_string(),
                None => {
                    println!("Debug: Path field is missing");
                    continue;
                }
            };
    
            let content = match retrieved_doc.get_first(self.content_field) {
                Some(content_field) => content_field.as_text().unwrap().to_string(),
                None => {
                    println!("Debug: Content field is missing");
                    continue;
                }
            };
    
            let line_end_indices_field = retrieved_doc.get_first(self.line_end_indices_field);
    
            let line_end_indices: Vec<u32> = match line_end_indices_field {
                Some(field) => {
                    match field.as_bytes() {
                        Some(bytes) => {
                            bytes.chunks_exact(4).map(|c| {
                                u32::from_le_bytes([c[0], c[1], c[2], c[3]])
                            }).collect()
                        }
                        None => {
                            println!("Debug: Failed to get bytes");
                            continue;
                        }
                    }
                }
                None => {
                    println!("Debug: Line end indices field is missing");
                    continue;
                }
            };
    
            for (mut line_number, window) in line_end_indices.windows(2).enumerate() {
                if let [start, end] = *window {
                    let line = &content[start as usize..end as usize];
    
                    if line.contains(query_str) {
                        line_number += 2;
                        let column = line.find(query_str).unwrap();
                        let context_start = line_number - 2;
                        let context_end = usize::min(line_number - 1, line_end_indices.len() - 1);
                        let context: String = line_end_indices[context_start..=context_end]
                            .windows(2)
                            .map(|w| {
                                let start = w[0] as usize;
                                let end = w[1] as usize;
                                &content[start..end]
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
    
                        results.push(SearchResult {
                            path: path.clone(),
                            line_number,
                            column,
                            context,
                        });
                    }
                }
            }
        }
    
        Ok(results)
    }

    pub fn format_fuzzy_search_results(results: Vec<SearchResult>) -> String {
        if results.is_empty() {
            return "No results found".to_string();
        }
    
        let mut formatted_results = String::new();
        for result in results {
            formatted_results.push_str(&format!(
                "File: {}, Line: {}, Column: {}, \nContent:\n{}\n\n",
                result.path, result.line_number, result.column, result.context
            ));
        }
        formatted_results
    }
    
    
    pub fn format_search_results(results: Vec<SearchResult>) -> String {
        if results.is_empty() {
            return "No results found".to_string();
        }
    
        let mut formatted_results = String::new();
        for result in results {
            formatted_results.push_str(&format!(
                "File: {}, Line: {}, Column: {}, \nContent:\n{}\n\n",
                result.path, result.line_number, result.column, result.context
            ));
        }
        formatted_results
    }
    
    pub fn load_all_documents(&self, lang: &str) -> Result<Vec<ContentDocument>> {
        let searcher = self.reader.searcher();

        let mut documents = Vec::new();
        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader.get_store_reader(0)?;
            let alive_bitset = segment_reader.alive_bitset();

            for doc in store_reader.iter(alive_bitset) {
                let doc = doc?;
                let lang_field_value = doc.get_first(self.lang_field)
                    .and_then(|f| f.as_text())
                    .unwrap_or("").to_lowercase();

                // println!("{:?} {:?}", lang_field_value, lang);

                if lang_field_value == lang {
                    let content = doc.get_first(self.content_field)
                        .and_then(|f| f.as_text())
                        .unwrap_or("")
                        .to_string();

                    let relative_path = doc.get_first(self.path_field)
                        .and_then(|f| f.as_text())
                        .unwrap_or("")
                        .to_string();

                    let line_end_indices: Vec<u32> = doc.get_first(self.line_end_indices_field)
                        .and_then(|f| f.as_bytes())
                        .unwrap_or(&[])
                        .chunks_exact(4)
                        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect();

                    let symbol_locations: SymbolLocations = doc.get_first(self.symbol_locations_field)
                        .and_then(|f| f.as_bytes())
                        .and_then(|b| bincode::deserialize(b).ok())
                        .unwrap_or_default();

                    // println!("{:?}", symbol_locations);

                    documents.push(ContentDocument {
                        content,
                        lang: Some(lang.to_string()),
                        relative_path,
                        line_end_indices,
                        symbol_locations,
                    });
                }
            }
        }

        Ok(documents)
    }


    pub fn line_word_to_byte_range(&self, content: &str, line_end_indices: &[u32], line_number: usize, word_start_index: usize, word_end_index: usize) -> Result<(usize, usize)> {
        if line_number == 0 || line_number > line_end_indices.len() {
            return Err(anyhow::anyhow!("Invalid line number"));
        }
    
        // Calculate the start and end byte indices for the line
        let start_of_line = if line_number == 1 {
            0
        } else {
            line_end_indices[line_number - 2] as usize + 1
        };
    
        let end_of_line = line_end_indices[line_number - 1] as usize;
    
        // Extract the line as a &str
        let line = &content[start_of_line..end_of_line];
    
        // println!("{}", line);
    
        // Validate word start and end indices
        if word_start_index >= word_end_index || word_end_index > line.chars().count() {
            return Err(anyhow::anyhow!("Invalid word indices"));
        }
    
        // Find the byte index for the start of the word
        let word_start_byte_index = line.chars().take(word_start_index).map(|c| c.len_utf8()).sum::<usize>();
    
        // Find the byte index for the end of the word
        let word_end_byte_index = line.chars().take(word_end_index).map(|c| c.len_utf8()).sum::<usize>();
    
        let start_byte = start_of_line + word_start_byte_index;
        let end_byte = start_of_line + word_end_byte_index;
    
        println!("{:?}", &content[start_byte..end_byte]);
    
        Ok((start_byte, end_byte))
    }

    fn detect_language(path: &Path) -> &'static str {
        let extension = path.extension().and_then(std::ffi::OsStr::to_str).unwrap_or("");
        TSLanguage::from_extension(extension).unwrap_or("plaintext")
    }

    pub fn token_info(&self, relative_path: &str, line: usize, start_index: usize, end_index: usize) -> Result<Vec<FileSymbols>> {
        let lang = Self::detect_language(Path::new(relative_path)).to_lowercase();

        // println!("{}", lang);

        let all_docs = self.load_all_documents(&lang)?;
        
        // Find the source document based on the provided relative path
        let source_document_idx = all_docs.iter().position(|doc| doc.relative_path == relative_path)
            .ok_or(anyhow::anyhow!("Source document not found"))?;
        
        let doc = all_docs.get(source_document_idx).unwrap();
    
        // Convert line number and indices to byte range
        let (start_byte, end_byte) = Self::line_word_to_byte_range(self, &doc.content, &doc.line_end_indices, line, start_index, end_index)?;

        let token = Token {
            relative_path,
            start_byte,
            end_byte,
        };
    
        let context = CodeNavigationContext {
            token,
            all_docs: &all_docs,
            source_document_idx,
            snipper: None,
        };
    
        let mut data = context.token_info();

        // Adjust line numbers by 1
        for file_symbols in &mut data {
            for occurrence in &mut file_symbols.data {
                occurrence.range.start.line += 1;
                occurrence.range.end.line += 1;
            }
        }
        
        Ok(data)
    }

    // New function to format token info results
    pub fn format_token_info(token_info_results: Vec<FileSymbols>) -> String {
        if token_info_results.is_empty() {
            return "No results found".to_string();
        }
    
        let mut formatted_results = String::new();
        for file_symbols in token_info_results {
            for occurrence in file_symbols.data {
                formatted_results.push_str(&format!(
                    "Kind: {}, File: {}, Line: {}, Column: {}\nContent:\n{}\n\n",
                    if let OccurrenceKind::Reference = occurrence.kind {"Reference"} else {"Definition"},
                    file_symbols.file,
                    occurrence.range.start.line,
                    occurrence.range.start.column,
                    occurrence.snippet.data,
                ));
            }
        }
        formatted_results
    }

    pub fn get_hoverable_ranges(&self, relative_path: &str) -> Result<Vec<TextRange>> {
        let lang = Self::detect_language(Path::new(relative_path)).to_lowercase();
        let all_docs = self.load_all_documents(&lang)?;
        
        // Find the document based on the provided relative path
        let doc = all_docs.iter().find(|doc| doc.relative_path == relative_path)
            .ok_or(anyhow::anyhow!("Document not found"))?;
        
        doc.hoverable_ranges().ok_or(anyhow::anyhow!("Hoverable ranges not found"))
    }

    pub fn format_hoverable_ranges(ranges: Vec<TextRange>) -> Vec<HashMap<String, u32>> {
        let mut formatted_ranges = Vec::new();
        for range in ranges {
            let mut range_map = HashMap::new();
            range_map.insert("start_line".to_string(), range.start.line as u32);
            range_map.insert("start_column".to_string(), range.start.column as u32);
            range_map.insert("end_line".to_string(), range.end.line as u32);
            range_map.insert("end_column".to_string(), range.end.column as u32);
            formatted_ranges.push(range_map);
        }
        formatted_ranges
    }
}

#[cfg(test)]
mod tests {
    use crate::Indexes;

    use super::*;

    #[tokio::test]
    async fn test_searcher_with_test_files() -> Result<()> {
        let root_path = Path::new("./test_files");
        let index_path = Path::new("./test_files/index");
        
        // Clean up the index directory if it exists
        if index_path.exists() {
            std::fs::remove_dir_all(index_path)?;
        }

        // Create indexes
        let buffer_size_per_thread = 60_000_000;
        let num_threads = 4;

        let indexes = Indexes::new(index_path, buffer_size_per_thread, num_threads).await?;
        indexes.index(root_path).await?;

        // Create a searcher and perform a search
        let searcher = Searcher::new(index_path)?;
        let result = searcher.text_search("indexes", true)?;

        // Print out the results (or you can write assertions here)
        for res in result {
            println!(
                "File: {}, Line: {}, Column: {}, Context: {}",
                res.path, res.line_number, res.column, res.context
            );
        }

        Ok(())
    }
}