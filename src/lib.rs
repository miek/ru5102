extern crate crc16;
extern crate serial;

use crc16::{State,MCRF4XX};
use std::result::Result;

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

impl Reader {
    pub fn new(port: &str) -> Reader {
        Reader { port: serial::open(port).unwrap() }
    }

    fn crc(data: &[u8]) -> u16 {
        State::<MCRF4XX>::calculate(data)
    }

    pub fn inventory(&self) -> Result<(), ()> {
        let cmd = Command { address: 0, command: CommandType::Inventory, data: Vec::new() };
        let pkt = cmd.to_bytes();
        println!("Packet: {:?}", pkt);
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
}
