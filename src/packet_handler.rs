#![allow(dead_code)]
#![allow(unused_variables)]

use crate::serial::Serial;

fn compute_checksum(id: u8, length: u8, instruction: u8, parameters: &[u8]) -> u8 {
    // https://emanual.robotis.com/docs/en/dxl/protocol1/#checksum-instruction-packet
    let mut checksum: u16 = 0; // avoid overflows, so set as u16
    checksum += id as u16;
    checksum += length as u16;
    checksum += instruction as u16;
    for param in parameters {
        checksum += *param as u16;
    }
    (!checksum & 0xff) as u8
}

enum Instruction {
    Ping,
    Read,
    Write,
    RegWrite,
    Action,
    SyncWrite,
    SyncRead,
}

impl Instruction {
    fn length(&self) -> u8 {
        // TODO: Do we want to do this like this?
        // It should be able to calculate it by itself by counting something,
        // I'm just not sure what it is counting yet
        match self {
            Instruction::Ping => 2,
            Instruction::Read => todo!(),
            Instruction::Write => todo!(),
            Instruction::RegWrite => todo!(),
            Instruction::Action => todo!(),
            Instruction::SyncWrite => todo!(),
            Instruction::SyncRead => todo!(),
        }
    }
}

impl From<Instruction> for u8 {
    fn from(value: Instruction) -> Self {
        match value {
            Instruction::Ping => 1,
            Instruction::Read => 2,
            Instruction::Write => 3,
            Instruction::RegWrite => 4,
            Instruction::Action => 5,
            Instruction::SyncWrite => 0x83,
            Instruction::SyncRead => 0x82,
        }
    }
}

struct InstructionPacket {
    // https://emanual.robotis.com/docs/en/dxl/protocol1/#instruction-packet
    // header0: u8,
    // header1: u8,
    id: u8,
    length: u8,
    instruction: u8,
    parameters: Vec<u8>,
    checksum: u8,
}

impl InstructionPacket {
    fn new(id: u8, length: u8, instruction: u8) -> Self {
        let parameters: Vec<u8> = vec![]; // TODO: add parameters
        Self {
            // header0: 0xff,
            // header1: 0xff,
            id,
            length,
            instruction,
            checksum: compute_checksum(id, length, instruction, &parameters),
            parameters,
        }
    }

    fn get_total_packet_length(&self) -> u32 {
        // "Header0, Header1, ID, Length" is added to the length of the packet
        self.length as u32 + 4
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![
            0xFF, // The first 2 bytes are always 0xff.
            0xFF, // AKA. "Header"
            self.id,
            self.length,
            self.instruction,
        ];
        bytes.extend_from_slice(&self.parameters);
        bytes.push(self.checksum);
        bytes
    }
}

pub struct StatusPacket {
    // https://emanual.robotis.com/docs/en/dxl/protocol1/#status-packetreturn-packet
    id: u8,
    length: u8,
    error: u8,
    params: Vec<u8>,
    checksum: u8,
}

impl StatusPacket {
    fn new(header: &[u8], id: u8, length: u8, error: u8, params: &[u8], checksum: u8) -> Self {
        assert!(header == [0xFF, 0xFF]);
        let computed_checksum = compute_checksum(id, length, error, params);
        assert!(checksum == computed_checksum); // TODO: handle this

        Self {
            id,
            length,
            error,
            params: params.to_vec(),
            checksum,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum Endianness {
    Little,
    Big,
}

#[derive(PartialEq, Eq, Debug)]
pub enum TxResult {
    Success,
    PortBusy,
    TxFail,
    TxError,
    NotAvailable,
}

pub enum RxResult {
    Success(Option<StatusPacket>),
    PortBusy,
    RxFail,
    RxWaiting,
    RxTimeout,
    RxCorrupt,
    NotAvailable,
}

#[derive(Debug)]
pub struct PacketHandler {
    endianness: Endianness,
    port: Serial,
}

impl PacketHandler {
    pub fn new(port_name: &str, baud_rate: u32) -> Self {
        Self {
            endianness: Endianness::Little,
            port: Serial::new(port_name, baud_rate).expect("error connecting to serial port"),
        }
    }
    pub fn ping(&mut self, motor_id: u8) -> RxResult {
        // TODO: Length is hardcoded here
        let tx_packet = InstructionPacket::new(motor_id, 2, Instruction::Ping.into());
        self.tx_rx_packet(tx_packet)
    }

    fn tx_rx_packet(&mut self, packet: InstructionPacket) -> RxResult {
        let result = self.tx_packet(&packet);
        if result != TxResult::Success {
            // Eh?
            return RxResult::RxFail;
        }
        if packet.id == 0xFE {
            // WARNING : Status Packet will not be returned if Broadcast ID(0xFE) is used.
            return RxResult::Success(None);
        }
        self.rx_packet()
    }

    fn tx_packet(&mut self, packet: &InstructionPacket) -> TxResult {
        if packet.get_total_packet_length() > 250 {
            return TxResult::TxError;
        }
        match self.port.write(&packet.as_bytes()) {
            Ok(_) => TxResult::Success,
            Err(_) => TxResult::TxFail,
        }
    }

    fn rx_packet(&mut self) -> RxResult {
        let mut header: [u8; 2] = [0; 2];
        self.port
            .read_exact(&mut header)
            .expect("reading header failed"); // TODO
        assert!(header == [0xFF, 0xFF]); // TODO
        let mut packet: [u8; 3] = [0; 3];
        self.port
            .read_exact(&mut packet)
            .expect("reading packet contents failed"); // TODO
        let param_len = packet[1];
        let mut params: Vec<u8> = Vec::with_capacity(param_len.into());
        self.port
            .read_exact(&mut params)
            .expect("reading param contents failed"); // TODO
        let mut checksum: [u8; 1] = [0; 1];
        self.port
            .read_exact(&mut checksum)
            .expect("reading checksum contents failed"); // TODO
        let status_packet = StatusPacket::new(
            &header,
            packet[0],
            packet[1],
            packet[2],
            &params,
            checksum[0],
        );
        RxResult::Success(Some(status_packet))
    }
}
