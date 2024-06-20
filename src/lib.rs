pub mod file;
pub mod indexes;
pub mod intelligence;
pub mod repository;
pub mod sync_handle;
pub mod symbol;
pub mod text_range;
pub mod search;
pub mod schema;
pub mod snippet;
pub mod content_document;

use std::path::Path;

pub use file::File;
pub use indexes::{Indexes, Indexable};
pub use repository::Repository;
use search::Searcher;
pub use sync_handle::SyncHandle;

use pyo3::prelude::*;
use serde_json::json;


/// Formats the sum of two numbers as string.
#[pyfunction]
fn go_to(root_path_str: &str, index_path_str: &str, relative_path: &str, line: usize, start_index: usize, end_index: usize) -> PyResult<String> {
    let root_path = Path::new(root_path_str);

    if !root_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Root path does not exist"));
    }

    let index_path = Path::new(index_path_str);
    
    if !index_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Index path does not exist"));
    }
    
    let buffer_size_per_thread = 15_000_000;
    let num_threads = 4;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Internal error: Failed to create Tokio runtime: {}", e))
    })?;
    
    rt.block_on(async {
        let indexes = Indexes::new(&index_path, buffer_size_per_thread, num_threads).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create indexes: {}", e))
        })?;
        
        indexes.index(root_path).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to index repository: {}", e))
        })?;

        let searcher = Searcher::new(&index_path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create searcher: {}", e))
        })?;
        
        let result = searcher.token_info(relative_path, line, start_index, end_index).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Error retrieving token info: {}", e))
        })?;
        
        Ok(search::Searcher::format_token_info(result))
    })
}

#[pyfunction]
fn text_search(root_path_str: &str, index_path_str: &str, query: &str, case_sensitive: bool) -> PyResult<String> {
    let root_path = Path::new(root_path_str);

    if !root_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Root path does not exist"));
    }

    let index_path = Path::new(index_path_str);

    if !index_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Index path does not exist"));
    }
    
    let buffer_size_per_thread = 15_000_000;
    let num_threads = 4;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Internal error: Failed to create Tokio runtime: {}", e))
    })?;
    
    rt.block_on(async {
        let indexes = Indexes::new(&index_path, buffer_size_per_thread, num_threads).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create indexes: {}", e))
        })?;
        
        indexes.index(root_path).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to index repository: {}", e))
        })?;

        let searcher = Searcher::new(&index_path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create searcher: {}", e))
        })?;
        
        let result = searcher.text_search(query, case_sensitive).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Error performing text search: {}", e))
        })?;
        
        Ok(search::Searcher::format_search_results(result))
    })
    // Ok("dsf");
}

#[pyfunction]
fn fuzzy_search(root_path_str: &str, index_path_str: &str, query: &str, max_distance: u8) -> PyResult<String> {
    let root_path = Path::new(root_path_str);

    if !root_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Root path does not exist"));
    }

    let index_path = Path::new(index_path_str);

    if !index_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Index path does not exist"));
    }
    
    let buffer_size_per_thread = 15_000_000;
    let num_threads = 4;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Internal error: Failed to create Tokio runtime: {}", e))
    })?;
    
    rt.block_on(async {
        let indexes = Indexes::new(&index_path, buffer_size_per_thread, num_threads).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create indexes: {}", e))
        })?;
        
        indexes.index(root_path).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to index repository: {}", e))
        })?;

        let searcher = Searcher::new(&index_path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create searcher: {}", e))
        })?;
        
        let result = searcher.fuzzy_search(query, max_distance).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Error performing fuzzy search: {}", e))
        })?;
        
        Ok(search::Searcher::format_fuzzy_search_results(result))
    })
}


#[pyfunction]
fn get_hoverable_ranges(root_path_str: &str, index_path_str: &str, relative_path: &str) -> PyResult<String> {
    let root_path = Path::new(root_path_str);

    if !root_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Root path does not exist"));
    }

    let index_path = Path::new(index_path_str);

    if !index_path.exists() {
        return Err(pyo3::exceptions::PyRuntimeError::new_err("Internal error: Index path does not exist"));
    }
    
    let buffer_size_per_thread = 15_000_000;
    let num_threads = 4;

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Internal error: Failed to create Tokio runtime: {}", e))
    })?;
    
    rt.block_on(async {
        let indexes = Indexes::new(&index_path, buffer_size_per_thread, num_threads).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create indexes: {}", e))
        })?;
        
        indexes.index(root_path).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to index repository: {}", e))
        })?;

        let searcher = Searcher::new(&index_path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create searcher: {}", e))
        })?;
        
        let ranges = searcher.get_hoverable_ranges(relative_path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Error retrieving hoverable ranges: {}", e))
        })?;
        
        let formatted_ranges = search::Searcher::format_hoverable_ranges(ranges);
        
        Ok(json!(formatted_ranges).to_string())
    })
}

#[pymodule]
fn code_nav_devon(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(go_to, m)?)?;
    m.add_function(wrap_pyfunction!(text_search, m)?)?;
    m.add_function(wrap_pyfunction!(fuzzy_search, m)?)?;
    m.add_function(wrap_pyfunction!(get_hoverable_ranges, m)?)?;
    Ok(())
}