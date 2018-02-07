extern crate crc16;
extern crate serial;

use crc16::{State,MCRF4XX};
use serial::core::prelude::*;
use std::io;
use std::time::Duration;
use std::result::Result;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Program(String),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<String> for Error {
    fn from(e: String) -> Error {
        Error::Program(e)
    }
}

pub struct Reader {
   port: serial::SystemPort, 
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum CommandType {
    Inventory = 0x01
}

#[derive(PartialEq, Debug)]
struct Command {
    address: u8,
    command: CommandType,
    data: Vec<u8>
}

impl Command {
    fn to_bytes(&self) -> Vec<u8> {
        let pkt_len = (self.data.len() + 4) as u8;
        let mut pkt: Vec<u8> = Vec::new();
        pkt.push(pkt_len);
        pkt.push(self.address);
        pkt.push(self.command as u8);
        pkt.append(&mut self.data.clone());
        let crc = Reader::crc(&pkt);
        pkt.push((crc & 0xFF) as u8);
        pkt.push(((crc >> 8) & 0xFF) as u8);
        pkt
    }
}

#[derive(PartialEq, Debug)]
struct Response {
    address: u8,
    command: u8,
    status: u8,
    data: Vec<u8>
}

impl Response {
    fn from_bytes(bytes: &[u8]) -> Response {
        assert_eq!(bytes[0] as usize, bytes.len() - 1);
        let data_len = bytes.len() - 6;
        let mut data: Vec<u8> = Vec::with_capacity(data_len);
        for i in 4..(data_len+4) {
            data.push(bytes[i]);
        }
        Response{
            address: bytes[1],
            command: bytes[2],
            status: bytes[3],
            data: data
        }
    }
}

impl Reader {
    pub fn new(port: &str) -> Result<Reader, Error> {
        let mut port = serial::open(port).unwrap();
        port.reconfigure(&|settings| {
            try!(settings.set_baud_rate(serial::Baud57600));
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            settings.set_flow_control(serial::FlowNone);
            Ok(())
        }).map_err(|e| format!("Failed to configure serial port: {}", e))?;

        port.set_timeout(Duration::from_millis(1000))
            .map_err(|e| format!("Failed to set serial port timeout: {}", e))?;
        Ok(Reader { port: port })
    }

    fn crc(data: &[u8]) -> u16 {
        State::<MCRF4XX>::calculate(data)
    }

    fn send_receive(&mut self, cmd: &[u8]) -> Result<Vec<u8>, Error> {
        std::io::Write::write(&mut self.port, &cmd)?;
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
        Ok(response)
    }

    pub fn inventory(&mut self) -> Result<(), ()> {
        let cmd = Command { address: 0, command: CommandType::Inventory, data: Vec::new() };
        let cmd = cmd.to_bytes();
        println!("Command: {:?}", cmd);
        let response = &self.send_receive(&cmd).unwrap();
        let response = Response::from_bytes(response);
        println!("Response: {:?}", response);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc() {
        assert_eq!(Reader::crc(b"abcdef"), 64265)
    }

    #[test]
    fn test_command() {
        assert_eq!(
            Command{
                address: 10,
                command: CommandType::Inventory,
                data: Vec::new()
            }.to_bytes(),
            [4, 10, 0x01, 171, 182]
        );
    }

    #[test]
    fn test_response() {
        assert_eq!(
            Response::from_bytes(&[5, 0, 1, 0, 1, 1]),
            Response{
                address: 0,
                command: 1,
                status: 0,
                data: Vec::new()
            }
        );
    }
}
