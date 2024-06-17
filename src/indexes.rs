use std::{fs, path::Path};
use anyhow::{Context, Result};
use async_trait::async_trait;
use tantivy::{schema::Schema, Index, IndexReader, IndexWriter};
use tokio::sync::Mutex;
use crate::file::File;

#[async_trait]
pub trait Indexable: Send + Sync {
    async fn index_repository(&self, root_path: &Path, writer: &IndexWriter) -> Result<()>;
    fn schema(&self) -> Schema;
}

pub struct IndexWriteHandle<'a> {
    source: &'a dyn Indexable,
    reader: &'a IndexReader,
    writer: IndexWriter,
}

impl<'a> IndexWriteHandle<'a> {
    pub async fn index(&self, root_path: &Path) -> Result<()> {
        self.source.index_repository(root_path, &self.writer).await
    }

    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        self.writer.rollback()?;
        Ok(())
    }
}

pub struct Indexer<T> {
    pub source: T,
    pub index: Index,
    pub reader: IndexReader,
    pub buffer_size: usize,
    pub threads: usize,
}

impl<T: Indexable> Indexer<T> {
    fn write_handle(&self) -> Result<IndexWriteHandle<'_>> {
        Ok(IndexWriteHandle {
            source: &self.source,
            reader: &self.reader,
            writer: self.index.writer_with_num_threads(self.threads, self.buffer_size * self.threads)?,
        })
    }

    fn init_index(schema: Schema, path: &Path, threads: usize) -> Result<Index> {
        fs::create_dir_all(path).context("failed to create index dir")?;
        let mut index = Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;
        index.set_multithread_executor(threads)?;
        Ok(index)
    }

    pub fn create(source: T, path: &Path, buffer_size: usize, threads: usize) -> Result<Self> {
        match Self::init_index(source.schema(), path, threads) {
            Ok(index) => {
                let reader = index.reader()?;
                Ok(Self {
                    reader,
                    index,
                    source,
                    threads,
                    buffer_size,
                })
            },
            Err(e) if e.to_string().contains("Schema error: 'An index exists but the schema does not match.'") => {
                // Delete the index directory
                fs::remove_dir_all(path)?;
                // Retry creating the Indexer instance
                let index = Self::init_index(source.schema(), path, threads)?;
                let reader = index.reader()?;
                Ok(Self {
                    reader,
                    index,
                    source,
                    threads,
                    buffer_size,
                })
            },
            Err(e) => Err(e),
        }
    }
}


pub struct Indexes {
    pub file: Indexer<File>,
    pub write_mutex: Mutex<()>,
}

impl Indexes {
    pub async fn new(index_path: &Path, buffer_size: usize, threads: usize) -> Result<Self> {
        Ok(Self {
            file: Indexer::create(File::new(), index_path, buffer_size, threads)?,
            write_mutex: Mutex::new(()),
        })
    }

    pub async fn index(&self, root_path: &Path) -> Result<()> {
        let _write_lock = self.write_mutex.lock().await;
        let mut writer = self.file.write_handle()?;
        writer.index( root_path).await?;
        writer.commit()?;
        Ok(())
    }
}
