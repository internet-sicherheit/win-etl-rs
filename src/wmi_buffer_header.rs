use std::io::{Error, Read, Seek};

use bitflags::bitflags;

/// The Header of a WMI_BUFFER (WMI_BUFFER_HEADER)
///
/// While the structure is not documented publicly,
/// this implementation aims to represent the header of a WMI_BUFFER as close as possible.
///
/// Reference: [Geoff Chappell, WMI_BUFFFER_HEADER](https://www.geoffchappell.com/studies/windows/km/ntoskrnl/inc/api/ntwmi/wmi_buffer_header/index.htm)
// The reverse engineering work of Geoff Chappell was used as a reference
#[derive(Debug, Clone)]
pub struct Header {
    pub node_header: NodeHeader,
    pub offset: u32,
    pub buffer_flags: BufferFlags,
    pub buffer_type: BufferType,
}

impl Header {
    pub fn parse<T: Read + Seek>(buf: &mut T) -> Result<Header, Error> {
        let node_header = NodeHeader::parse(buf)?;

        let mut bytes = [0u8; 4];
        buf.read_exact(&mut bytes)?;
        let offset = u32::from_le_bytes(bytes);

        let mut bytes = [0u8; 2];
        buf.read_exact(&mut bytes)?;
        let flag_bits = u16::from_le_bytes(bytes);
        let buffer_flags = BufferFlags::from_bits(flag_bits).ok_or(Error::new(
            std::io::ErrorKind::InvalidData,
            "oh no, can't parse WMI_BUFFER_HEADER, unknown buffer flag encounterd!",
        ))?;

        let mut bytes = [0u8; 2];
        buf.read_exact(&mut bytes)?;
        let buffer_type: BufferType = u16::from_le_bytes(bytes).try_into()?;

        Ok(Header {
            node_header,
            offset,
            buffer_flags,
            buffer_type,
        })
    }

    pub fn get_buffer_size(&self) -> u32 {
        self.node_header.buffer_size
    }
}

type Timestamp = u64;

/// Representation of the WNODE_HEADER which is part of a [WMI_BUFFER_HEADER](Header)
#[derive(Debug, Clone)]
pub struct NodeHeader {
    pub buffer_size: u32,
    pub saved_offset: u32,
    pub current_offset: u32,
    pub reference_count: u32,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
    pub clock: Clock,
    pub processor: ProcessorInfo,
    pub logger_id: u16,
    pub state: State,
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ClockType {
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
pub struct Clock {
    pub clock_type: ClockType,
    pub frequency: u64,
}
impl From<u64> for Clock {
    /// Convert from a 64-bit bitfield to [Clock]
    fn from(value: u64) -> Self {
        // The two MSBits are the Type, the rest is the frequency.
        const MASK: u64 = 0b11 << 62;
        let clk_type_bits = ((value & MASK) >> 62) as u8;
        let freq = value & !MASK;

        // Since we extracted the first two bits with a mask its ok to transmute here.
        let clk_type: ClockType = unsafe { core::mem::transmute(clk_type_bits) };

        Clock {
            clock_type: clk_type,
            frequency: freq,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_clock_parsing() {
        use super::Clock;
        use super::ClockType;

        /// 0xC0 -> two MSBits are set -> 0x03 variant of ClockType -> CpuCycleCounter
        let test_data: u64 = 0xC000_0000_0000_0010;

        let clock: Clock = test_data.into();

        assert_eq!(&clock.clock_type, &ClockType::CpuCycleCounter);
        assert_eq!(&clock.frequency, &0x10);
    }
}

#[derive(Debug, Clone)]
pub enum ProcessorInfo {
    Aligned(AlignedProcessorNumber),
    Indexed(u16),
}
impl From<u16> for ProcessorInfo {
    fn from(value: u16) -> Self {
        ProcessorInfo::Indexed(value)
    }
}

#[derive(Debug, Clone)]
pub struct AlignedProcessorNumber {
    pub processor_number: u8,
    pub alignment: u8,
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct State: u32 {
        const Flush = 0x1;
        const InUse = 0x2;
        const Free = 0x4;
    }
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct BufferFlags: u16 {
        const FlushMarker = 0x01;
        const EventsLost = 0x02;
        const BufferLost = 0x04;
        const RealtimeBackupCorrupt = 0x08;
        const RealtimeBackup = 0x10;
        const ProcessorIndex = 0x20;
        const Compressed = 0x40;
    }
}

#[derive(Debug, Clone)]
#[repr(u16)]
pub enum BufferType {
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
impl TryFrom<u16> for BufferType {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        use BufferType::*;
        match value {
            0x0 => Ok(Generic),
            0x1 => Ok(Rundown),
            0x2 => Ok(CtxSwap),
            0x3 => Ok(RefTime),
            0x4 => Ok(Header),
            0x5 => Ok(Batched),
            0x6 => Ok(EmptyMarker),
            0x7 => Ok(DbgInfo),
            0x8 => Ok(Maximum),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "oh no, encountered unknown variant for WMI_BUFFER_HEADER BufferType!",
            )),
        }
    }
}
