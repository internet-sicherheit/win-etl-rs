use std::io::{Error, Read, Seek};

use bitflags::bitflags;

pub struct Buffer {
    pub header: Header,
    pub content: WmiBufferContent,
}

pub struct WmiBufferContent;

pub struct Header {
    node_header: NodeHeader,
    offset: u32,
    buffer_flags: BufferFlags,
    buffer_type: BufferType,
}

impl Header {
    pub fn parse<T: Read + Seek>(buf: &mut T) -> Result<Header, Error> {
        let node_header = NodeHeader::parse(buf)?;
        todo!();
    }
}

type Timestamp = u64;

/// Representation of the WNODE_HEADER which is part of a [WMI_BUFFER_HEADER](Header)
#[derive(Debug, Clone)]
struct NodeHeader {
    buffer_size: u32,
    saved_offset: u32,
    current_offset: u32,
    reference_count: u32,
    timestamp: Timestamp,
    sequence_number: u64,
    clock: Clock,
    processor: ProcessorInfo,
    logger_id: u16,
    state: State,
}

impl NodeHeader {
    fn parse<T: Read + Seek>(buf: &mut T) -> Result<NodeHeader, Error> {
        let mut bytes = [0u8; 48];
        buf.read_exact(&mut bytes)?;

        // TODO check how to handle endianness correctly
        // TODO check for different versions of WNODE_HEADER
        let buf_size = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let saved_offset = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let current_offset = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let ref_count = u32::from_le_bytes(bytes[12..16].try_into().unwrap());

        let timestamp = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let seq_number = u64::from_le_bytes(bytes[24..32].try_into().unwrap());

        let clock: Clock = u64::from_le_bytes(bytes[32..40].try_into().unwrap()).into();

        let processor: ProcessorInfo = u16::from_le_bytes(bytes[40..42].try_into().unwrap()).into();
        let logger_id = u16::from_le_bytes(bytes[42..44].try_into().unwrap());

        let state_bits = u32::from_le_bytes(bytes[44..48].try_into().unwrap());
        let state = State::from_bits(state_bits).ok_or(Error::new(
            std::io::ErrorKind::InvalidData,
            "oh no, can't parse WNODE_HEADER, unknown state flag encounterd!",
        ))?;

        Ok(NodeHeader {
            buffer_size: buf_size,
            saved_offset,
            current_offset,
            reference_count: ref_count,
            timestamp,
            sequence_number: seq_number,
            clock,
            processor,
            logger_id,
            state,
        })
    }
}

#[derive(Debug, Clone)]
#[repr(u8)]
enum ClockType {
    /// Default clock or not specified
    Default = 0x0,
    /// Query Performance Counter (QPC)
    QueryPerformanceCounter = 0x1,
    /// Windows System Time
    SystemTime = 0x2,
    /// CPU Cycle Counter
    CpuCycleCounter = 0x3,
}

#[derive(Debug, Clone)]
struct Clock {
    clock_type: ClockType,
    frequency: u64,
}
impl From<u64> for Clock {
    fn from(value: u64) -> Self {
        const MASK: u64 = 0b11 << 62;
        let clk_type_bits = ((value & MASK) >> 62) as u8;
        let freq = value & !MASK;

        let clk_type: ClockType = unsafe { core::mem::transmute(clk_type_bits) };

        Clock {
            clock_type: clk_type,
            frequency: freq,
        }
    }
}

#[derive(Debug, Clone)]
enum ProcessorInfo {
    Aligned(AlignedProcessorNumber),
    Indexed(u16),
}
impl From<u16> for ProcessorInfo {
    fn from(value: u16) -> Self {
        ProcessorInfo::Indexed(value)
    }
}

#[derive(Debug, Clone)]
struct AlignedProcessorNumber {
    processor_number: u8,
    alignment: u8,
}

bitflags! {
    #[derive(Debug, Clone)]
    struct State: u32 {
        const Flush = 0x1;
        const InUse = 0x2;
        const Free = 0x4;
    }
}

bitflags! {
    struct BufferFlags: u16 {
        const FlushMarker = 0x01;
        const EventsLost = 0x02;
        const BufferLost = 0x04;
        const RealtimeBackupCorrupt = 0x08;
        const RealtimeBackup = 0x10;
        const ProcessorIndex = 0x20;
        const Compressed = 0x40;
    }
}

enum BufferType {
    /// Normal WMI-Buffer / ETL-Chunk
    Generic = 0x0,
    /// Buffer containing only Rundown-Events
    Rundown = 0x1,
    CtxSwap = 0x2,
    /// Indicates a realtime ETW session
    RefTime = 0x3,
    /// Marks the first ETL-Chunk, containing header information
    Header = 0x4,
    Batched = 0x5,
    EmptyMarker = 0x6,
    DbgInfo = 0x7,
    Maximum = 0x8,
}
