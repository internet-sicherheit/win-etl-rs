#![doc = include_str!("../README.md")]
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom};

pub mod wmi_buffer_header;
use wmi_buffer_header::Header;

/// Event Trace Log (ETL)
///
/// Every ETL consists of a number of [ETL-chunks](EtlChunk) which are in the form of WMI_BUFFER structures.
pub struct Etl {
    pub chunks: Vec<EtlChunk>,
}

impl Etl {
    /// Load a ETL from a buffer (e.g. a file)
    pub fn from_buf<T: Read + Seek>(mut buf: T) -> Result<Etl, Error> {
        let mut etl = Etl { chunks: Vec::new() };
        loop {
            let pos = buf.stream_position()?;

            let res = Header::parse(&mut buf);
            if res
                .as_ref()
                .is_err_and(|e| e.kind() == ErrorKind::UnexpectedEof)
            {
                break;
            }

            let header = res?;
            let seek = SeekFrom::Start(pos + header.get_buffer_size() as u64);
            buf.seek(seek)?;

            etl.chunks.push(EtlChunk {
                header,
                content: WmiBufferContent,
                start: pos,
            })
        }
        Ok(etl)
    }
}

/// A chunk from an ETL file
///
/// Each Chunk is in the form of a WMI_BUFFER.
/// A WMI_BUFFER always starts with a [WMI_BUFFER_HEADER](Header) followed by event objects.
pub struct EtlChunk {
    /// Header of the ETL-chunk
    pub header: Header,
    /// Content (events) in this chunk
    pub content: WmiBufferContent,
    /// Denotes the start of the chunk in the ETL file
    pub start: u64,
}

/// Placeholder for the buffer content (to be implemented)
pub struct WmiBufferContent;
