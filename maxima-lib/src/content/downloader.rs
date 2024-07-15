use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
    prelude,
    sync::{Arc, Mutex},
    task,
};

use anyhow::Result;
use async_compression::tokio::write::DeflateDecoder;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt, TryStreamExt};
use log::{debug, error, warn};
use reqwest::Client;
use strum_macros::Display;
use tokio::{
    fs::{create_dir, create_dir_all, File},
    io::{AsyncWrite, BufReader, BufWriter},
};

use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::{
    content::{
        zip::CompressionType,
        zlib::{restore_zlib_state, write_zlib_state},
    },
    util::hash::hash_file_crc32,
};

use super::zip::{ZipFile, ZipFileEntry};

/// 50mb chunks
const MAX_CHUNK_SIZE: i64 = 50_000_000;

#[async_trait]
trait DownloadDecoder {
    async fn save_state(&mut self, buf: &mut BytesMut);
    async fn restore_state(&mut self, buf: &mut Bytes);

    fn get_mut<'b>(&mut self) -> Arc<Mutex<dyn AsyncWriteWrapper>>;
}

struct ZLibDeflateDecoder {
    decoder: Arc<Mutex<DeflateDecoder<BufWriter<File>>>>,
}

impl ZLibDeflateDecoder {
    fn new(writer: BufWriter<File>) -> Self {
        Self {
            decoder: Arc::new(Mutex::new(DeflateDecoder::new(writer))),
        }
    }
}

#[async_trait]
impl DownloadDecoder for ZLibDeflateDecoder {
    async fn save_state(&mut self, buf: &mut BytesMut) {
        {
            let mut decoder = self.decoder.lock().unwrap();
            let zstream = decoder.inner_mut().decoder_mut().inner.decompress.get_raw();
            write_zlib_state(buf, zstream);
        }

        log::info!("Serialized zlib state");
    }

    async fn restore_state(&mut self, buf: &mut Bytes) {
        let mut decoder = self.decoder.lock().unwrap();
        let decompress = &mut decoder.inner_mut().decoder_mut().inner.decompress;
        decompress.reset(false);
        let zstream = decompress.get_raw();
        restore_zlib_state(buf, zstream);
        log::info!("Reset and restored zlib state");
    }

    fn get_mut(&mut self) -> Arc<Mutex<dyn AsyncWriteWrapper>> {
        self.decoder.clone()
    }
}

struct NoopDecoder {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl NoopDecoder {
    pub fn new(writer: BufWriter<File>) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

#[async_trait]
impl DownloadDecoder for NoopDecoder {
    async fn save_state(&mut self, buf: &mut BytesMut) {}
    async fn restore_state(&mut self, buf: &mut Bytes) {}

    fn get_mut<'b>(&mut self) -> Arc<Mutex<dyn AsyncWriteWrapper>> {
        self.writer.clone()
    }
}

trait AsyncWriteWrapper: AsyncWrite + Unpin {}
impl<T: AsyncWrite + Unpin> AsyncWriteWrapper for T {}

struct AsyncWriterWrapper {
    inner: Arc<Mutex<dyn AsyncWriteWrapper>>,
}

impl AsyncWriterWrapper {
    fn new(inner: Arc<Mutex<dyn AsyncWriteWrapper>>) -> Self {
        AsyncWriterWrapper { inner }
    }
}

impl AsyncWrite for AsyncWriterWrapper {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<prelude::v1::Result<usize, io::Error>> {
        Pin::new(&mut *self.inner.lock().unwrap()).poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<prelude::v1::Result<(), io::Error>> {
        Pin::new(&mut *self.inner.lock().unwrap()).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<prelude::v1::Result<(), io::Error>> {
        Pin::new(&mut *self.inner.lock().unwrap()).poll_shutdown(cx)
    }
}

struct DownloadChunk {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Display)]
pub enum DownloadError {
    DownloadFailed(usize),
    ChunkFailed,
}

impl std::error::Error for DownloadError {}

enum EntryDownloadState {
    Fresh,
    Complete,
    Resumable,
    Borked,
}

struct DownloadContext {
    path: PathBuf,
}

struct EntryDownloadRequest<'a> {
    context: &'a DownloadContext,
    url: &'a str,
    entry: &'a ZipFileEntry,
    client: Client,
    decoder: Box<dyn DownloadDecoder>,
}

impl<'a> EntryDownloadRequest<'a> {
    pub fn new(context: &'a DownloadContext, url: &'a str, entry: &'a ZipFileEntry, client: Client, decoder: Box<dyn DownloadDecoder>) -> Self {
        //let state = self.state

        Self { context, url, entry, client, decoder }
    }

    async fn state(context: &DownloadContext, entry: &ZipFileEntry) -> EntryDownloadState {
        let path = context.path.join(entry.name());
        if !path.exists() {
            return EntryDownloadState::Fresh;
        }

        let hash = match hash_file_crc32(path) {
            Ok(hash) => hash,
            Err(_) => {
                warn!("Failed to retrieve hash for file {}", entry.name());
                0
            }
        };

        let hash_match = entry.crc32() != &hash;
        let size_match = *entry.uncompressed_size() == File::open(path).await.unwrap().metadata().await.unwrap().len() as i64;

        if !size_match {
            return EntryDownloadState::Resumable;
        }

        EntryDownloadState::Complete
    }

    // async fn download(&mut self) {
    //     while self.chunks.len() > self.chunk as usize {
    //         log::info!("Downloading {}, chunk {}", self.entry.name(), self.chunk);

    //         self.download_chunk(self.chunk).await;

    //         self.decoder.save_state().await;
    //         self.chunk += 1;

    //         // For debugging
    //         //self.decoder.restore_state().await;
    //     }
    // }

    // pub async fn download_chunk(&mut self, chunk: u32) {
    //     let mut result = Err(DownloadError::ChunkFailed);
    //     while let Err(_) = result {
    //         let chunk = &self.chunks[chunk as usize];
    //         result = self.download_range(chunk.start, chunk.end).await;
    //     }
    // }

    /// End is not inclusive
    pub async fn download_range(&mut self, start: i64, end: i64) -> Result<(), DownloadError> {
        let offset = self.entry.data_offset();
        let range = format!("bytes={}-{}", offset + start as i64, offset + end - 1);

        let data = match self
            .client
            .get(self.url)
            .header("range", range)
            .send()
            .await
        {
            Ok(res) => res,
            Err(err) => {
                error!("Failed to download ({}): {}", self.entry.name(), err);
                return Err(DownloadError::ChunkFailed);
            }
        };

        let stream = data.bytes_stream();
        let counting_stream = ByteCountingStream::new(stream);
        let stream = counting_stream.into_async_read();
        let mut stream_reader = BufReader::new(stream.compat());

        let mut wrapper = AsyncWriterWrapper::new(self.decoder.get_mut());
        tokio::io::copy(&mut stream_reader, &mut wrapper)
            .await
            .unwrap();

        Ok(())
    }
}

pub struct ZipDownloader {
    url: String,
    client: Client,
    manifest: ZipFile,
}

impl ZipDownloader {
    pub async fn new(url: &str) -> Result<Self> {
        let manifest = ZipFile::fetch(url).await?;

        Ok(Self {
            url: url.to_owned(),
            client: Client::builder().build()?,
            manifest,
        })
    }

    pub fn manifest(&self) -> &ZipFile {
        &self.manifest
    }

    pub async fn download_single_file(
        &self,
        entry: &ZipFileEntry,
        bytes_downloaded: usize,
    ) -> Result<usize> {
        let dir_path = Path::new("DownloadTest");
        let file_path = dir_path.join(entry.name());

        if bytes_downloaded == 0 {
            if !file_path.parent().unwrap().exists() {
                warn!("Creating {}", file_path.parent().unwrap().display());
                create_dir_all(&file_path.parent().unwrap()).await?;
            }

            if entry.name().ends_with("/") && !file_path.exists() {
                // This is a folder, create the dir
                debug!("{} is a directory", entry.name());
                warn!("Creating {}", file_path.display());
                create_dir(file_path).await?;
                return Ok(0);
            }
        }

        if *entry.uncompressed_size() == 0 {
            debug!("{} is empty", entry.name());
            return Ok(0);
        }

        let offset = entry.data_offset();
        debug!("Type: {:?}", entry.compression_type());
        debug!("Compressed Size: {}", entry.compressed_size());
        debug!("Offset: {}", offset);

        let file = File::create(&file_path).await?;
        let writer = tokio::io::BufWriter::new(file);

        let decoder: Box<dyn DownloadDecoder> = match entry.compression_type() {
            CompressionType::None => Box::new(NoopDecoder::new(writer)),
            CompressionType::Deflate => Box::new(ZLibDeflateDecoder::new(writer)),
        };

        let context = DownloadContext {
            path: file_path
        };

        let mut request = EntryDownloadRequest::new(&context, &self.url, entry, self.client.clone(), decoder);
        //request.download().await;

        Ok(0)
    }
}

struct ByteCountingStream<S> {
    inner: S,
    byte_count: usize,
}

impl<S> ByteCountingStream<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
{
    fn new(inner: S) -> Self {
        ByteCountingStream {
            inner,
            byte_count: 0,
        }
    }

    fn byte_count(&self) -> usize {
        self.byte_count
    }
}

impl<S> Stream for ByteCountingStream<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<bytes::Bytes, tokio::io::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            std::task::Poll::Ready(Some(Ok(chunk))) => {
                self.byte_count += chunk.len();
                std::task::Poll::Ready(Some(Ok(chunk)))
            }
            std::task::Poll::Ready(Some(Err(_))) => std::task::Poll::Ready(Some(Err(
                futures::io::Error::other(DownloadError::DownloadFailed(self.byte_count)),
            ))),
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
