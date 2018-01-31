extern crate crc16;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate serial;

use bincode::{serialize, Bounded};
use crc16::{State,MCRF4XX};
use serde::ser::{Serialize, Serializer, SerializeTuple};
use std::result::Result;

pub struct Reader {
   port: serial::SystemPort, 
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum CommandType {
    Inventory = 0x01
}

impl Serialize for CommandType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
       serializer.serialize_u8(*self as u8)
    }
}

#[derive(Serialize, PartialEq, Debug)]
struct Command {
    address: u8,
    command: CommandType,
    #[serde(serialize_with = "serialize_command_data")]
    data: Vec<u8>
}

impl Command {
    fn to_bytes(&self) -> bincode::Result<Vec<u8>> {
        let pkt_len = (self.data.len() + 4) as u8;
        let serialize_len = (pkt_len - 2) as u64;
        let mut pkt = serialize(&self, Bounded(serialize_len))?;
        pkt.insert(0, pkt_len);
        let crc = Reader::crc(&pkt);
        let mut crc = serialize(&crc, Bounded(2))?;
        pkt.append(&mut crc);
        Ok(pkt)
    }
}

fn serialize_command_data<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let mut seq = serializer.serialize_tuple(data.len())?;
    for d in data {
        seq.serialize_element(&d)?;
    }
    seq.end()
}

impl Reader {
    pub fn new(port: &str) -> Reader {
        Reader { port: serial::open(port).unwrap() }
    }

    fn crc(data: &[u8]) -> u16 {
        State::<MCRF4XX>::calculate(data)
    }

    pub fn inventory(&self) -> bincode::Result<Vec<u8>> {
        let cmd = Command { address: 0, command: CommandType::Inventory, data: Vec::new() };
        let pkt = cmd.to_bytes()?;
        println!("Packet: {:?}", pkt);
        Ok(pkt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc() {
        assert_eq!(Reader::crc(b"abcdef"), 64265)
    }
}
