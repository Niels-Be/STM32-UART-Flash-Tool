use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    extended_erase_special,
    helper::{connect_port, toggle_reset, GpioPin},
    read_memory, verify_memory, write_memory, SpecialEraseType,
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
        let started = Instant::now();
        log::debug!(
            "Flash attempt started: {} bytes at {:#010X}",
            data.len(),
            self.config.address
        );
        let mut port = self
            .port
            .as_mut()
            .ok_or_else(|| {
                let e = std::io::Error::other("Port not open");
                log::error!("Flash aborted: {}", e);
                e
            })?;
        let erase_started = Instant::now();
        log::debug!("Mass erase started");
        // Note: this might time out for some reason, it does succeed anyway
        let res = extended_erase_special(port, SpecialEraseType::MassErase);
        if let Err(e) = res {
            log::debug!(
                "Mass erase failed after {:?}; resetting/reconnecting: {:?}",
                erase_started.elapsed(),
                e
            );
            // close current port
            drop(self.port.take());

            let reconnect_started = Instant::now();
            toggle_reset(&mut self.gpio_reset).map_err(|e| {
                log::error!("Flash aborted: post-erase reset failed after {:?}: {:?}", reconnect_started.elapsed(), e);
                e
            })?;
            self.port = Some(connect_port(&self.config.port, self.config.baud_rate).map_err(|e| {
                log::error!("Flash aborted: post-erase reconnect failed after {:?}: {:?}", reconnect_started.elapsed(), e);
                e
            })?);
            port = self.port.as_mut().unwrap();
            log::debug!("Post-erase reset/reconnect completed after {:?}", reconnect_started.elapsed());
        }
        log::debug!("Erase/recovery phase completed after {:?}", erase_started.elapsed());

        log::debug!("Flashing {} bytes to {:#010X}", data.len(), self.config.address);
        let write_started = Instant::now();
        write_memory(port, self.config.address, data).map_err(|e| {
            log::error!(
                "Flash aborted: write failed after {:?} (attempt elapsed {:?}): {:?}",
                write_started.elapsed(),
                started.elapsed(),
                e
            );
            e
        })?;
        log::debug!("Writing done after {:?}, verifying", write_started.elapsed());
        let verify_started = Instant::now();
        verify_memory(port, self.config.address, data).map_err(|e| {
            log::error!(
                "Flash aborted: verification failed after {:?} (attempt elapsed {:?}): {:?}",
                verify_started.elapsed(),
                started.elapsed(),
                e
            );
            e
        })?;
        log::info!(
            "Flash write/verification successful after {:?} (verification {:?})",
            started.elapsed(),
            verify_started.elapsed()
        );
        sleep(Duration::from_millis(100));
        Ok(())
    }

    pub fn reset(mut self) -> Result<(), std::io::Error> {
        let started = Instant::now();
        log::debug!("Resetting boot pin");
        self.port.take(); // close port
        let e1 = self.gpio_boot.set_value(0).map_err(|e| {
            log::debug!("Clearing boot pin failed after {:?}: {:?}", started.elapsed(), e);
            e
        });
        let e2 = toggle_reset(&mut self.gpio_reset).map_err(|e| {
            log::debug!("Application reset failed after {:?}: {:?}", started.elapsed(), e);
            e
        });
        let result = e1.and(e2);
        log::debug!("Boot-pin clear/application reset finished after {:?}: {:?}", started.elapsed(), result);
        result
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
        if self.port.is_some() {
            // if its not already closed close it now
            let f = std::mem::replace(self, Flasher::empty());
            if let Err(e) = f.reset() {
                log::error!("Error resetting flasher: {:?}", e);
            }
        }
    }
}
