//! Emulated UART 16550. (ref: https://wiki.osdev.org/Serial_Ports)

use super::PortIoDevice;
use crate::serial_print;

use rvm::{RvmError, RvmResult};

const DATA_REG: u16 = 0;
const INT_EN_REG: u16 = 1;
const FIFO_CTRL_REG: u16 = 2;
const LINE_CTRL_REG: u16 = 3;
const MODEM_CTRL_REG: u16 = 4;
const LINE_STATUS_REG: u16 = 5;
// const MODEM_STATUS_REG: u16 = 6;
const SCRATCH_REG: u16 = 7;

// const UART_FIFO_CAPACITY: usize = 16;

bitflags::bitflags! {
    /// Line status flags
    struct LineStsFlags: u8 {
        const INPUT_FULL = 1;
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5;
        // 6 and 7 unknown
    }
}

pub struct Uart16550 {
    port_base: u16,
}

impl PortIoDevice for Uart16550 {
    fn port_range(&self) -> core::ops::Range<u16> {
        self.port_base..self.port_base + 8
    }

    fn read(&self, _port: u16, _access_size: u8) -> RvmResult<u32> {
        Err(RvmError::Unsupported)
    }

    fn write(&self, port: u16, access_size: u8, value: u32) -> RvmResult {
        if access_size != 1 {
            log::error!("Invalid serial port I/O write size: {} != 1", access_size);
            return Err(RvmError::InvalidParam);
        }
        match port - self.port_base {
            DATA_REG => serial_print!("{}", value as u8 as char),
            INT_EN_REG | FIFO_CTRL_REG | LINE_CTRL_REG | MODEM_CTRL_REG | SCRATCH_REG => {
                log::info!("Unimplemented serial port I/O write: {:#x}", port); // unimplemented
            }
            LINE_STATUS_REG => {} // ignore
            _ => unreachable!(),
        }
        Ok(())
    }
}

impl Uart16550 {
    pub const fn new(port_base: u16) -> Self {
        Self { port_base }
    }
}
