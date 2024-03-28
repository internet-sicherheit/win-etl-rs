#![doc = include_str!("../README.md")]
use log::{debug, error, info, trace};
use std::{
    fmt::Display,
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Seek, SeekFrom},
};
use win_etw_event::EtwEvent;

pub mod trace_logfile_header;
pub mod wmi_buffer_header;

use trace_logfile_header::TraceLogfileHeader;
use wmi_buffer_header::Header;

use crate::wmi_buffer_header::{BufferType, WMI_BUFFER_CONTENT_OFFSET};

/// Event Trace Log (ETL)
///
/// Every ETL consists of a number of [ETL-chunks](EtlChunk) which are in the form of WMI_BUFFER structures.
pub struct Etl<F: Read + Seek> {
    pub header: TraceLogfileHeader,
    pub timestamp_scale: f64,
    pub timestamp_base: u64,
    chunk_size: u32,
    file: F,
}

impl<F: Read + Seek> Etl<F> {
    /// Load a ETL from a stream (e.g. a file)
    pub fn from_buf(mut buf: F) -> Result<Etl<F>, Error> {
        info!("Reading ETL from Stream");
        let first_chunk_header = Header::parse(&mut buf)?;
        if first_chunk_header.buffer_type != BufferType::Header {
            error!(
                "ETL file starts with buffer of type {:?}, expected BufferType::Header!",
                first_chunk_header.buffer_type
            );
            return Err(Error {
                kind: ErrorKind::MissingHeader,
                cause: "missing header buffer for etl file".into(),
            });
        }
        trace!("Read buffer header: {first_chunk_header:?}");

        buf.seek(SeekFrom::Start(WMI_BUFFER_CONTENT_OFFSET as u64))?;
        info!(
            "Found valid header-chunk, parsing header event at 0x{:X}",
            buf.stream_position().unwrap()
        );

        let event = win_etw_event::parse_header(&mut buf)?;
        let EtwEvent::SystemTraceEvent(header_system_trace_event) = event else {
            error!("Encountered wrong event format in header chunk!");
            return Err(Error {
                kind: ErrorKind::MissingHeader,
                cause: "missing header event for etl file".into(),
            });
        };
        debug!("Found SystemTraceEvent");
        trace!("SystemTraceEvent: {:#?}", &header_system_trace_event.header);
        let logfile_header =
            TraceLogfileHeader::parse_slice(header_system_trace_event.payload.as_slice())?;

        let timestamp_scale = logfile_header.get_timestamp_scale();
        let timestamp_base =
            logfile_header.get_timestamp_base(header_system_trace_event.header.system_time);
        let etl = Etl {
            header: logfile_header,
            chunk_size: first_chunk_header.get_buffer_size(),
            file: buf,
            timestamp_scale,
            timestamp_base,
        };
        Ok(etl)
    }

    /// Load all Chunk descriptors contained in the ETL
    pub fn chunks(&mut self) -> Result<Vec<EtlChunk>, Error> {
        self.file.seek(SeekFrom::Start(self.chunk_size as u64))?;
        let mut chunks = Vec::new();
        loop {
            let pos = self.file.stream_position()?;

            let res = Header::parse(&mut self.file);
            if res
                .as_ref()
                .is_err_and(|e| e.kind() == IoErrorKind::UnexpectedEof)
            {
                break;
            }

            let header = res?;
            let seek = SeekFrom::Start(pos + header.get_buffer_size() as u64);
            self.file.seek(seek)?;

            chunks.push(EtlChunk { header, start: pos })
        }
        Ok(chunks)
    }

    pub fn load_events(&mut self, chunk: &EtlChunk) -> Result<Vec<EtwEvent>, Error> {
        let seek = SeekFrom::Start(chunk.start + WMI_BUFFER_CONTENT_OFFSET as u64);
        self.file.seek(seek)?;
        trace!(
            "Reading events starting at 0x{:X}",
            self.file.stream_position()?
        );
        trace!(
            "Chunk data ends at 0x{:X}",
            (chunk.start + chunk.header.node_header.saved_offset as u64 - 1)
        );

        let mut events = Vec::new();
        loop {
            if self.file.stream_position()?
                >= (chunk.start + chunk.header.node_header.saved_offset as u64)
            {
                break;
            }
            match win_etw_event::parse_header(&mut self.file) {
                Ok(e) => {
                    trace!("Found event of type {:?}", e.get_event_type());
                    self.file.seek(SeekFrom::Current(e.padding() as i64))?;
                    events.push(e);
                }
                Err(_) => break,
            }
        }
        Ok(events)
    }
}

/// A chunk descriptor from an ETL file
///
/// Each Chunk is in the form of a WMI_BUFFER.
/// A WMI_BUFFER always starts with a [WMI_BUFFER_HEADER](Header) followed by event objects.
pub struct EtlChunk {
    /// Header of the ETL-chunk
    pub header: Header,
    /// Denotes the start of the chunk in the ETL file
    pub start: u64,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    cause: Box<dyn std::error::Error + 'static>,
}
impl Error {
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ErrorKind::IoError => write!(f, "{}", self.cause),
            ErrorKind::MissingHeader => write!(f, "missing required header"),
            ErrorKind::UsupportedEvent => write!(f, "a unsupported event type was found"),
            ErrorKind::ModernEventError => {
                write!(f, "failed to process modern event: {}", self.cause)
            }
            ErrorKind::CorruptedHeader => write!(f, "corrupted header: {}", self.cause),
            ErrorKind::UnsupportedMode => write!(f, "unsupported mode: {}", self.cause),
            ErrorKind::IncompatibleType => write!(f, "incompatible type: {}", self.cause),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    /// An error occured while reading data from a file/buffer
    IoError,
    /// The ETL is missing a required Header
    MissingHeader,
    /// ETL file header is corrupted
    CorruptedHeader,
    /// A unsupported event type was found
    UsupportedEvent,
    /// An error occured while processing a modern event
    ModernEventError,
    /// ETL has an unsupported mode
    UnsupportedMode,
    /// Conversion to a Rust type failed
    IncompatibleType,
}

impl From<win_etw_event::Error> for Error {
    fn from(value: win_etw_event::Error) -> Self {
        match value {
            win_etw_event::Error::Io(e) => Error {
                kind: ErrorKind::IoError,
                cause: Box::new(e),
            },
            win_etw_event::Error::ModernEvent(e) => Error {
                kind: ErrorKind::ModernEventError,
                cause: e.into(),
            },
            win_etw_event::Error::Unsupported => Error {
                kind: ErrorKind::UsupportedEvent,
                cause: "a unsupported event type was found".into(),
            },
        }
    }
}

impl From<IoError> for Error {
    fn from(value: IoError) -> Self {
        Error {
            kind: ErrorKind::IoError,
            cause: value.into(),
        }
    }
}
