//! Octo Virtual Sensors
//! 
//! Update the virtual sensors on an Aqua computer Octo
//!
//! Usage:
//! ```
//! use octo_virtual_sensors::Octo;
//! let mut octo = Octo::new().unwrap();
//! octo.update_virtual_sensors(&[1, 2, 3]).unwrap();
//! ```
//!
use anyhow::{Context, Result};
use rusb::{Device, DeviceList, GlobalContext};
use std::time::Duration;


/// Simple interface to update the 'Virtual sensors on the Aquacomputer Octo
pub struct Octo {
    device: Device<GlobalContext>,
    buffer: Vec<u8>,
}

/// Header offset
static HEADER: usize = 1;

impl Octo {
    /// Create a new Octo
    ///
    /// Tries to find the connected Octo. Fails if unable to find it based on vendor_id and product_id
    pub fn new() -> Result<Self> {
        for device in DeviceList::new().context("Getting USB Device list")?.iter() {
            let dd = &device.device_descriptor().context("Getting device ID")?;
            
            static VENDOR_ID: u16 = 3184;
            static PRODUCT_ID: u16 = 61457;

            if dd.vendor_id() == VENDOR_ID && dd.product_id() == PRODUCT_ID {
                let buffer = vec![
                    4, 127, 255, 127, 255, 127, 255, 127, 255, 127, 255, 127, 255, 127, 255, 127,
                    255, 127, 255, 127, 255, 127, 255, 127, 255, 127, 255, 127, 255, 127, 255, 127,
                    255, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255,
                ];
                return Ok(Self { device, buffer });
            }
        }
        anyhow::bail!("Could not find Aquastream Octo");
    }

    /// Update virtual sensors
    ///
    /// Takes a slice of sensor with each values index being used as
    /// the virtual sensors output number
    pub fn update_virtual_sensors(&mut self, sensor_values: &[u16]) -> Result<usize> {
        self.update_buffer(sensor_values);
        self.send()
    }

    /// Update the sensors values in the existing buffer
    fn update_buffer(&mut self, sensor_values: &[u16]) {
        for index in 0..16 {
            let sensor_offset = HEADER + 2 * index;            
            if let Some(value) = sensor_values.get(index) {
                let value = (100_u16 * value).to_be_bytes();
                self.buffer[sensor_offset] = value[0];
                self.buffer[sensor_offset + 1] = value[1];
            } else {
                let value = 32767_u16.to_be_bytes();
                self.buffer[sensor_offset] = value[0];
                self.buffer[sensor_offset + 1] = value[1];
            }
        }

        let crc_ = crc::Crc::<u16>::new(&crc::CRC_16_USB);
        let mut digest = crc_.digest();

        static CHECKSUM: usize = 2;
        digest.update(&self.buffer[HEADER..self.buffer.len() - CHECKSUM]);

        let checksum = digest.finalize().to_be_bytes();

        self.buffer[49] = checksum[0];
        self.buffer[50] = checksum[1];
    }

    /// Send the buffer to the device via a USB bulk write
    fn send(&mut self) -> Result<usize> {
        let open = self.device.open().context("Opening USB device")?;
        static TIMEOUT: Duration = Duration::from_secs(1);        
        open
            .write_bulk(2, &self.buffer, TIMEOUT)
            .context("Sending bulk transfer to Octo")
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use std::{process::Command, time::Duration};

    /// Test the buffer looks correct
    #[test]
    fn update_buffer() -> Result<()> {
        let mut octo = super::Octo::new()?;
        octo.update_buffer(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let expected = vec![
            4, 0, 100, 0, 200, 1, 44, 1, 144, 1, 244, 2, 88, 2, 188, 3, 32, 3, 132, 3, 232, 4, 76,
            4, 176, 5, 20, 5, 120, 5, 220, 6, 64, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            218, 118,
        ];
        for (expected, result) in expected.iter().zip(octo.buffer.iter()) {
            assert_eq!(expected, result);
        }
        Ok(())
    }

    /// Test sensors actually update
    #[test]
    fn update_virtual_sensors() -> Result<()> {
        let mut octo = super::Octo::new()?;
        octo.update_virtual_sensors(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])?;
        // Wait for sensors to update
        std::thread::sleep(Duration::from_secs(1));
        let sensors = Command::new("/usr/bin/sensors").output()?.stdout;
        let sensors = String::from_utf8(sensors)?;
        let sensors_txt = sensors
            .lines()
            .filter(|line| line.contains("Virtual sensor"))
            .map(|l| l.trim().to_string())
            .collect::<Vec<String>>();
        let expected = [
            "Virtual sensor 1:    +1.0°C",
            "Virtual sensor 2:    +2.0°C",
            "Virtual sensor 3:    +3.0°C",
            "Virtual sensor 4:    +4.0°C",
            "Virtual sensor 5:    +5.0°C",
            "Virtual sensor 6:    +6.0°C",
            "Virtual sensor 7:    +7.0°C",
            "Virtual sensor 8:    +8.0°C",
            "Virtual sensor 9:    +9.0°C",
            "Virtual sensor 10:  +10.0°C",
            "Virtual sensor 11:  +11.0°C",
            "Virtual sensor 12:  +12.0°C",
            "Virtual sensor 13:  +13.0°C",
            "Virtual sensor 14:  +14.0°C",
            "Virtual sensor 15:  +15.0°C",
            "Virtual sensor 16:  +16.0°C",
        ];
        for (expected, sensor) in expected.iter().zip(sensors_txt.iter()) {
            assert_eq!(expected, sensor);
        }
        Ok(())
    }
}
