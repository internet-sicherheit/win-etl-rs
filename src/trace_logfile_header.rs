use crate::ErrorKind;

use super::Error;
use bitflags::bitflags;
use byteorder::{ByteOrder, LittleEndian};
use log::{debug, trace};
use num_enum::TryFromPrimitive;
use std::io::{Read, Seek};
use widestring::ucstring::U16CString;

/// Header Event describing a Event Trace Log (ETL)
///
/// A TraceLogfileHeader is a synthetic [SYSTEM_TRACE_EVENT](win_etw_event::system_trace_event::SystemTraceEvent).
///
/// <https://www.geoffchappell.com/studies/windows/km/ntoskrnl/inc/shared/evntrace/trace_logfile_header.htm>
#[derive(Clone, Debug)]
#[repr(C)]
pub struct TraceLogfileHeader {
    pub buffer_size: u32,
    version: Version,
    pub provider_version: u32,
    pub number_of_processors: u32,
    /// _WIP Placeholder_
    pub end_time: u64,
    pub time_resolution: u32,
    pub max_file_size: u32,
    pub log_file_mode: LogFileMode,
    pub buffer_written: u32,
    pub start_buffers: u32,
    pub pointer_size: u32,
    pub events_lost: u32,
    pub cpu_speed_mhz: u32,
    pub clock_interrupt_source: ClockInterruptSource,
    pub perf_freq: u64,
    pub start_time: u64,
    pub clock_type: ClockType,
    pub logger_name: String,
    pub log_file_name: String,
}

/// Version stored in the [TraceLogfileHeader]
///
/// Can be either in the form of [VersionTuple] or u32, depending on the system the ETL was created on.
#[derive(Clone, Copy)]
#[repr(C)]
union Version {
    version_tuple: VersionTuple,
    version: u32,
}
impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version = unsafe { self.version };
        write!(f, "Version: {version:08X}")
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VersionTuple {
    major_version: u8,
    minor_version: u8,
    sub_version: u8,
    sub_minor_version: u8,
}

// TODO complete Variants
// https://learn.microsoft.com/en-us/windows/win32/etw/logging-mode-constants
bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct LogFileMode: u32 {
        const RealTimeMode = 0x0000_0100;
        const StopOnHybridShutdown = 0x0040_0000;
        const PersistOnHybridShutdown = 0x0080_0000;
        const CompressedMode = 0x4000_0000;
        const NoPerProcessorBuffering = 0x1000_0000;
        const IndependentSessionMode = 0x0800_0000;
    }
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum ClockInterruptSource {
    /// not specified
    HalPlatformTimerNotSpecified = 0x0,
    /// Programable Interrupt Timer
    HalPlatformTimerPIT8252 = 0x1,
    /// Real Time Clock
    HalPlatformTimerRTC = 0x2,
    /// Advanced Configuration and Power Interface
    HalPlatformTimerACPI = 0x3,
    /// Advanced Configuration and Power Interface (Broken)
    HalPlatformTimerACPIBroken = 0x4,
    /// High Performance Event Timer
    HalPlatformTimerHPET = 0x5,
    /// CPU Performance Counter
    HalPlatformTimerProcessorCounter = 0x6,
    /// Hypervisor Reference Timer
    HalPlatformTimerHVReferenceTimer = 0x7,
    /// Simple Firmware Interface
    HalPlatformTimerSFI = 0x8,
    /// Advanced Programable Interrupt Controller
    HalPlatformTimerAPIC = 0x9,
    /// Synthetic Software Time inside Hypervisor
    HalPlatformTimerHVSynthetic = 0xA,
    /// Custom
    HalPlatformTimerCustom = 0xB,
    /// Cycle Counter
    HalPlatformTimerCycleCounter = 0xC,
    /// Global Interrupt Timer
    HalPlatformTimerGIT = 0xD,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive, Eq, PartialEq)]
#[repr(u32)]
pub enum ClockType {
    QueryPerformanceCounter = 0x1,
    SystemTime = 0x2,
    CpuCycleCounter = 0x3,
}

impl TraceLogfileHeader {
    /// Parse a TraceLogfileHeader from a reader
    ///
    /// Takes 280 bytes and omits parsing of logger name and log file name.
    pub fn parse<T: Read + Seek>(reader: &mut T) -> Result<TraceLogfileHeader, Error> {
        let mut bytes = [0u8; 280];
        reader.read_exact(&mut bytes)?;
        trace!("280 bytes of TraceLogfileHeader read");
        TraceLogfileHeader::parse_slice(&bytes)
    }
    /// Parse a TraceLogfileHeader from a slice
    ///
    /// Take a slice with atleast 280 bytes and parses it into a TraceLogfileHeader.
    pub fn parse_slice(bytes: &[u8]) -> Result<TraceLogfileHeader, Error> {
        if bytes.len() < 280 {
            return Err(Error {
                kind: ErrorKind::CorruptedHeader,
                cause: "not enough bytes to construct TraceLogfileHeader".into(),
            });
        }
        trace!("TRACE_LOGFILE_HEADER bytes: {:02X?}", &bytes);

        let buffer_size = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let version = Version {
            version: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        };
        let provider_version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let number_of_processors = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let end_time = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let time_resolution = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let max_file_size = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let log_file_mode =
            LogFileMode::from_bits(u32::from_le_bytes(bytes[32..36].try_into().unwrap()))
                .ok_or_else(|| {
                    debug!(
                        "Unexpected LogFileMode: 0x{:08X}",
                        u32::from_le_bytes(bytes[32..36].try_into().unwrap())
                    );
                    Error {
                        kind: ErrorKind::UnsupportedMode,
                        cause: "unknown LogFileMode encounterd".into(),
                    }
                })?;
        // .ok_or(Error::new(
        //     std::io::ErrorKind::InvalidData,
        //     "oh no, unknown LogFileMode encounterd!",
        // ))?;
        let buffer_written = u32::from_le_bytes(bytes[36..40].try_into().unwrap());

        let clock_interrupt_source =
            ClockInterruptSource::try_from(u64::from_le_bytes(bytes[56..64].try_into().unwrap()))
                .map_err(|_| Error {
                kind: ErrorKind::UnsupportedMode,
                cause: "unknown clock interrupt source".into(),
            })?;

        let start_buffers = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        let pointer_size = u32::from_le_bytes(bytes[44..48].try_into().unwrap());
        let events_lost = u32::from_le_bytes(bytes[48..52].try_into().unwrap());
        let cpu_speed_mhz = u32::from_le_bytes(bytes[52..56].try_into().unwrap());

        let perf_freq = u64::from_le_bytes(bytes[256..264].try_into().unwrap());
        let start_time = u64::from_le_bytes(bytes[264..272].try_into().unwrap());

        let clock_type = ClockType::try_from(u32::from_le_bytes(
            bytes[272..276].try_into().unwrap(),
        ))
        .map_err(|_| Error {
            kind: ErrorKind::UnsupportedMode,
            cause: "unknown clock type".into(),
        })?;

        let mut u16_vec: Vec<u16> = vec![0; bytes[280..].len() / 2];
        if bytes[280..].len() % 2 != 0 {
            debug!("Uneven length of buffer for u16Cstrings");
        } else {
            LittleEndian::read_u16_into(&bytes[280..], u16_vec.as_mut_slice());
        }

        let cstring_logger_name = U16CString::from_vec_truncate(u16_vec.as_slice());
        let logger_name = cstring_logger_name.to_string().map_err(|e| Error {
            kind: ErrorKind::IncompatibleType,
            cause: e.into(),
        })?;

        let cstring_log_file_name =
            U16CString::from_vec_truncate(u16_vec.split_off(cstring_logger_name.len() + 1));
        let log_file_name = cstring_log_file_name.to_string().map_err(|e| Error {
            kind: ErrorKind::IncompatibleType,
            cause: e.into(),
        })?;

        Ok(TraceLogfileHeader {
            buffer_size,
            version,
            provider_version,
            number_of_processors,
            end_time,
            time_resolution,
            max_file_size,
            log_file_mode,
            buffer_written,
            logger_name,
            log_file_name,
            clock_interrupt_source,
            start_buffers,
            pointer_size,
            events_lost,
            cpu_speed_mhz,
            perf_freq,
            clock_type,
            start_time,
        })
    }

    pub fn get_timestamp_scale(&self) -> f64 {
        match self.clock_type {
            ClockType::QueryPerformanceCounter => 10_000_000.0 / self.perf_freq as f64,
            ClockType::SystemTime => 1.0,
            ClockType::CpuCycleCounter => 10.0 / self.cpu_speed_mhz as f64,
        }
    }

    pub fn get_timestamp_base(&self, system_time: u64) -> u64 {
        let scale = self.get_timestamp_scale();
        self.start_time - (scale * system_time as f64) as u64
    }
}
