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

type Timestamp = u64;

struct NodeHeader {
    buffer_size: u32,
    saved_offset: u32,
    current_offset: u32,
    reference_count: u32,
    timestamp: Timestamp,
    sequence_number: u64,
    clock: Clock,
    client_context: EtwBufferContext,
    state: State,
}

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

struct Clock {
    clock_type: ClockType,
    frequency: u64,
}

struct EtwBufferContext;

bitflags! {
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
