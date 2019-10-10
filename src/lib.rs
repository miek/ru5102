//! Driver for the CF-RU5102 UHF RFID reader
extern crate crc16;
extern crate num_enum;
extern crate serial;
extern crate failure;

pub mod error;

use crc16::{State, MCRF4XX};
use num_enum::TryFromPrimitive;
use serial::core::prelude::*;
use std::convert::TryFrom;
use std::time::Duration;

use crate::error::{Error, Result};

pub struct Reader {
    port: serial::SystemPort,
    address: u8,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[allow(dead_code)]
enum CommandType {
    // EPC C1 G2（ISO18000-6C) Commands
    Inventory = 0x01,
    ReadData = 0x02,
    WriteData = 0x03,
    WriteEPC = 0x04,
    KillTag = 0x05,
    Lock = 0x06,
    BlockErase = 0x07,
    ReadProtect = 0x08,
    ReadProtectWithoutEPC = 0x09,
    ResetReadProtect = 0x0a,
    CheckReadProtect = 0x0b,
    EASAlarm = 0x0c,
    CheckEASAlarm = 0x0d,
    BlockLock = 0x0e,
    InventorySingle = 0x0f,
    BlockWrite = 0x10,

    // ISO18000-6B Commands
    InventorySignal6B = 0x50,
    InventoryMultiple6B = 0x51,
    ReadData6B = 0x52,
    WriteData6B = 0x53,
    CheckLock6B = 0x54,
    Lock6B = 0x55,

    // Reader Commands
    GetReaderInformation = 0x21,
    SetRegion = 0x22,
    SetAddress = 0x24,
    SetScanTime = 0x25,
    SetBaudRate = 0x28,
    SetPower = 0x2F,
    AcoustoOpticControl = 0x33,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ResponseStatus {
    OK = 0x00,
    ReturnBeforeInventoryFinished = 0x01,
    ScanTimeOverflow = 0x02,
    MoreData = 0x03,
    ReaderFlashFull = 0x04,
    AccessPasswordError = 0x05,
    KillTagError = 0x09,
    KillPasswordZero = 0x0A,
    CommandNotSupported = 0x0B,

    SaveFail = 0x13,
    CannotAdjust = 0x14,

    // TODO: there are more of these
    CommandExecuteError = 0xF9,
    PoorCommunication = 0xFA,
    NoTags = 0xFB,
    TagError = 0xFC,
    WrongLength = 0xFD,
    IllegalCommand = 0xFE,
    ParameterError = 0xFF,
}

impl ResponseStatus {
    fn is_success(&self) -> bool {
        match self {
            ResponseStatus::OK => true,
            ResponseStatus::ReturnBeforeInventoryFinished => true,
            ResponseStatus::ScanTimeOverflow => true,
            ResponseStatus::MoreData => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Debug)]
struct Command {
    address: u8,
    command: CommandType,
    data: Vec<u8>,
}

impl Command {
    fn to_bytes(&self) -> Vec<u8> {
        let pkt_len = (self.data.len() + 4) as u8;
        let mut pkt: Vec<u8> = Vec::new();
        pkt.push(pkt_len);
        pkt.push(self.address);
        pkt.push(self.command as u8);
        pkt.append(&mut self.data.clone());
        let crc = Reader::calculate_crc(&pkt);
        pkt.push((crc & 0xFF) as u8);
        pkt.push(((crc >> 8) & 0xFF) as u8);
        pkt
    }
}

#[derive(PartialEq, Debug)]
struct Response {
    address: u8,
    command: u8,
    status: ResponseStatus,
    data: Vec<u8>,
}

impl Response {
    fn from_bytes(bytes: &[u8]) -> Result<Response> {
        assert_eq!(bytes[0] as usize, bytes.len() - 1);
        let len = bytes.len();

        let crc = Reader::calculate_crc(&bytes[0..len - 2]);
        let payload_crc: u16 = ((bytes[len - 1] as u16) << 8) + bytes[len - 2] as u16;
        if payload_crc != crc {
            return Err(Error::Program("Bad CRC".to_string()));
        }

        let payload = &bytes[1..len - 2];
        Ok(Response {
            address: payload[0],
            command: payload[1],
            status: ResponseStatus::try_from(payload[2]).unwrap(),
            data: payload[3..].to_vec(),
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct ReaderInformation {
    version: Vec<u8>,
    reader_type: u8,
    supported_protocols: u8,
    max_freq: u8,
    min_freq: u8,
    power: u8,
    scan_time: u8,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MemoryLocation {
    Password = 0x00,
    EPC = 0x01,
    TID = 0x02,
    User = 0x03,
}

#[derive(PartialEq, Debug)]
pub struct ReadCommand {
    pub epc: Vec<u8>,
    pub location: MemoryLocation,
    pub start_address: u8,
    pub count: u8,
    pub password: Option<Vec<u8>>,
    pub mask_address: Option<u8>,
    pub mask_length: Option<u8>,
}

impl ReadCommand {
    fn to_bytes(&self) -> Vec<u8> {
        let mut pkt: Vec<u8> = Vec::new();
        // EPC size is in words, which are 2 bytes long
        pkt.push(self.epc.len() as u8 / 2);
        pkt.extend(self.epc.clone());
        pkt.push(self.location as u8);
        pkt.push(self.start_address);
        pkt.push(self.count);
        if let Some(p) = &self.password {
            pkt.extend(p.clone());
        } else {
            pkt.extend(vec![0, 0, 0, 0]);
        }
        if let Some(addr) = self.mask_address {
            pkt.push(addr);
        }
        if let Some(len) = self.mask_length {
            pkt.push(len);
        }
        pkt
    }
}

#[derive(PartialEq, Debug)]
pub struct WriteCommand {
    pub epc: Vec<u8>,
    pub location: MemoryLocation,
    pub start_address: u8,
    pub data: Vec<u8>,
    pub password: Option<Vec<u8>>,
    pub mask_address: Option<u8>,
    pub mask_length: Option<u8>,
}

impl WriteCommand {
    fn to_bytes(&self) -> Vec<u8> {
        let mut pkt: Vec<u8> = Vec::new();
        // EPC and write size is in words, which are 2 bytes long
        pkt.push(self.data.len() as u8 / 2);
        pkt.push(self.epc.len() as u8 / 2);
        pkt.extend(self.epc.clone());
        pkt.push(self.location as u8);
        pkt.push(self.start_address);
        pkt.extend(self.data.clone());
        if let Some(p) = &self.password {
            pkt.extend(p.clone());
        } else {
            pkt.extend(vec![0, 0, 0, 0]);
        }
        if let Some(addr) = self.mask_address {
            pkt.push(addr);
        }
        if let Some(len) = self.mask_length {
            pkt.push(len);
        }
        pkt
    }
}

#[derive(PartialEq, Debug)]
pub struct KillCommand {
    pub epc: Vec<u8>,
    pub password: Vec<u8>,
    pub mask_address: Option<u8>,
    pub mask_length: Option<u8>,
}

impl KillCommand {
    fn to_bytes(&self) -> Vec<u8> {
        let mut pkt: Vec<u8> = Vec::new();
        // EPC and write size is in words, which are 2 bytes long
        pkt.push(self.epc.len() as u8 / 2);
        pkt.extend(self.epc.clone());
        pkt.extend(self.password.clone());
        if let Some(addr) = self.mask_address {
            pkt.push(addr);
        }
        if let Some(len) = self.mask_length {
            pkt.push(len);
        }
        pkt
    }
}

impl ReaderInformation {
    fn from_bytes(bytes: &[u8]) -> ReaderInformation {
        assert_eq!(bytes.len(), 8);
        ReaderInformation {
            version: bytes[0..2].to_vec(),
            reader_type: bytes[2],
            supported_protocols: bytes[3],
            max_freq: bytes[4],
            min_freq: bytes[5],
            power: bytes[6],
            scan_time: bytes[7],
        }
    }
}

impl Reader {
    pub fn new(port: &str) -> Result<Reader> {
        let mut port = serial::open(port)
            .map_err(|e| format!("Unable to connect to serial port {}: {:?}", port, e))?;
        port.reconfigure(&|settings| {
            try!(settings.set_baud_rate(serial::Baud57600));
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            settings.set_flow_control(serial::FlowNone);
            Ok(())
        })
        .map_err(|e| format!("Failed to configure serial port: {}", e))?;

        port.set_timeout(Duration::from_millis(1000))
            .map_err(|e| format!("Failed to set serial port timeout: {}", e))?;
        Ok(Reader {
            port: port,
            address: 0,
        })
    }

    fn calculate_crc(data: &[u8]) -> u16 {
        State::<MCRF4XX>::calculate(data)
    }

    fn send_receive(&mut self, cmd: Command) -> Result<Response> {
        let cmd_bytes = cmd.to_bytes();
        std::io::Write::write(&mut self.port, &cmd_bytes)?;
        let mut len = [0u8; 1];
        std::io::Read::read_exact(&mut self.port, &mut len)?;
        let len = len[0];
        let mut response: Vec<u8> = Vec::with_capacity(len as usize + 1);
        response.push(len);
        {
            use std::io::Read;
            let reference = self.port.by_ref();
            reference.take(len as u64).read_to_end(&mut response)?;
        }
        let response = Response::from_bytes(&response)?;
        Ok(response)
    }

    /// Fetch information on the reader in a ReaderInformation structure
    pub fn reader_information(&mut self) -> Result<ReaderInformation> {
        let cmd = Command {
            address: self.address,
            command: CommandType::GetReaderInformation,
            data: Vec::new(),
        };
        let response = self.send_receive(cmd)?;
        if !response.status.is_success() {
            return Err(Error::from(response.status));
        }
        Ok(ReaderInformation::from_bytes(&response.data))
    }

    /// Inventory all tags in the reader's range.
    ///
    /// Returns a vector of tag IDs.
    pub fn inventory(&mut self) -> Result<Vec<Vec<u8>>> {
        let cmd = Command {
            address: self.address,
            command: CommandType::Inventory,
            data: Vec::new(),
        };
        let response = self.send_receive(cmd)?;

        if response.status == ResponseStatus::NoTags {
            return Ok(vec![]);
        } else if !response.status.is_success() {
            return Err(Error::from(response.status));
        }

        let num_tags = response.data[0];
        let mut offset = 1;
        let mut tags = Vec::new();

        for _i in 0..num_tags {
            let tag_len = response.data[offset];
            offset += 1;
            tags.push(response.data[offset..(offset + tag_len as usize)].to_vec());
            offset += tag_len as usize;
        }

        Ok(tags)
    }

    pub fn read_data(&mut self, read_cmd: ReadCommand) -> Result<Vec<u8>> {
        let cmd = Command {
            address: self.address,
            command: CommandType::ReadData,
            data: read_cmd.to_bytes(),
        };
        let response = self.send_receive(cmd)?;

        if !response.status.is_success() {
            return Err(Error::from(response.status));
        }

        Ok(response.data)
    }

    pub fn write_data(&mut self, write_cmd: WriteCommand) -> Result<()> {
        let cmd = Command {
            address: self.address,
            command: CommandType::WriteData,
            data: write_cmd.to_bytes(),
        };
        let response = self.send_receive(cmd)?;

        if !response.status.is_success() {
            return Err(Error::from(response.status));
        }

        Ok(())
    }

    /// Send a KillCommand
    pub fn kill(&mut self, kill_cmd: KillCommand) -> Result<()> {
        let cmd = Command {
            address: self.address,
            command: CommandType::KillTag,
            data: kill_cmd.to_bytes(),
        };
        let response = self.send_receive(cmd)?;

        if !response.status.is_success() {
            return Err(Error::from(response.status));
        }
        Ok(())
    }
}

#[test]
fn test_crc() {
    assert_eq!(Reader::calculate_crc(b"abcdef"), 64265)
}

#[test]
fn test_command() {
    assert_eq!(
        Command {
            address: 10,
            command: CommandType::Inventory,
            data: Vec::new()
        }
        .to_bytes(),
        [4, 10, 0x01, 171, 182]
    );
}

#[test]
fn test_response() {
    assert_eq!(
        Response::from_bytes(&[5, 0, 1, 251, 242, 61]).unwrap(),
        Response {
            address: 0,
            command: 1,
            status: ResponseStatus::NoTags,
            data: Vec::new()
        }
    );
}
