use std::{thread::sleep, time::Duration};

use crate::{
    extended_erase_special,
    helper::{connect_port, toggle_reset, GpioPin},
    read_memory, write_memory, SpecialEraseType,
};

#[derive(Debug, Clone)]
pub struct FlashConfig {
    pub port: String,
    pub baud_rate: u32,
    pub boot_pin: u32,
    pub reset_pin: u32,
    pub address: u32,
}

impl<T> From<T> for FlashConfig
where
    T: Into<String>,
{
    fn from(t: T) -> Self {
        FlashConfig {
            port: t.into(),
            ..Default::default()
        }
    }
}

impl Default for FlashConfig {
    fn default() -> Self {
        FlashConfig {
            port: "/dev/ttyHS1".to_string(),
            baud_rate: 115200,
            boot_pin: 9,
            reset_pin: 8,
            address: 0x08000000,
        }
    }
}

pub struct Flasher {
    config: FlashConfig,
    port: Option<Box<dyn serialport::SerialPort>>,
    gpio_boot: GpioPin,
    gpio_reset: GpioPin,
}

impl Flasher {
    fn empty() -> Self {
        Flasher {
            config: FlashConfig::default(),
            port: None,
            gpio_boot: GpioPin::None,
            gpio_reset: GpioPin::None,
        }
    }

    pub fn open(config: FlashConfig) -> Result<Self, std::io::Error> {
        log::debug!("Setting boot pin {}", config.boot_pin);
        let mut gpio_boot = GpioPin::new(config.boot_pin)?;
        gpio_boot.set_value(1)?;
        sleep(Duration::from_millis(100));

        log::debug!("Toggling reset pin {}", config.reset_pin);
        let mut gpio_reset = GpioPin::new(config.reset_pin)?;
        toggle_reset(&mut gpio_reset)?;

        let port = connect_port(&config.port, config.baud_rate)?;
        log::debug!("Connected on {}", config.port);

        Ok(Flasher {
            config,
            port: Some(port),
            gpio_boot,
            gpio_reset,
        })
    }

    pub fn flash(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        let mut port = self
            .port
            .as_mut()
            .ok_or(std::io::Error::other("Port not open"))?;
        // Note: this might time out for some reason, it does succeed anyway
        let res = extended_erase_special(port, SpecialEraseType::MassErase);
        if let Err(e) = res {
            log::debug!("Reconnect after erase: {:?}", e);
            // close current port
            drop(self.port.take());

            toggle_reset(&mut self.gpio_reset)?;
            self.port = Some(connect_port(&self.config.port, self.config.baud_rate)?);
            port = self.port.as_mut().unwrap();
        }

        log::debug!("Flashing {} bytes to {}", data.len(), self.config.address);
        write_memory(port, self.config.address, data)?;
        log::debug!("Flash Complete");
        sleep(Duration::from_millis(100));
        Ok(())
    }

    pub fn reset(mut self) -> Result<(), std::io::Error> {
        log::debug!("Resetting boot pin");
        let e1 = self.gpio_boot.set_value(0);
        let e2 = toggle_reset(&mut self.gpio_reset);
        std::mem::forget(self);
        e1.and(e2)
    }

    pub fn read_memory(&mut self, address: u32, dst_data: &mut [u8]) -> Result<(), std::io::Error> {
        let port = self
            .port
            .as_mut()
            .ok_or(std::io::Error::other("Port not open"))?;
        read_memory(port, address, dst_data)
    }
}

impl Drop for Flasher {
    fn drop(&mut self) {
        let f = std::mem::replace(self, Flasher::empty());
        if let Err(e) = f.reset() {
            log::error!("Error resetting flasher: {:?}", e);
        }
    }
}
