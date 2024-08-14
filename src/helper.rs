use std::{thread::sleep, time::Duration};

use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use serialport::prelude::*;

use crate::{extended_erase_special, flasher::FlashConfig, write_memory, SpecialEraseType};

pub fn full_process_flash(data: &[u8], conf: &FlashConfig) -> Result<(), std::io::Error> {
    log::debug!("Setting boot pin {}", conf.boot_pin);
    let mut gpio_boot = GpioPin::new(conf.boot_pin)?;
    gpio_boot.set_value(1)?;
    sleep(Duration::from_millis(100));

    log::debug!("Toggling reset pin {}", conf.reset_pin);
    let mut gpio_reset = GpioPin::new(conf.reset_pin)?;
    toggle_reset(&mut gpio_reset)?;

    let mut port = connect_port(&conf.port, conf.baud_rate)?;
    log::debug!("Connected on {}", conf.port);

    // Note: this might time out for some reason, it does succeed anyway
    let res = extended_erase_special(&mut port, SpecialEraseType::MassErase);
    if let Err(e) = res {
        log::debug!("Reconnect after erase: {:?}", e);
        // close current port
        drop(port);

        toggle_reset(&mut gpio_reset)?;
        port = connect_port(&conf.port, conf.baud_rate)?;
    }

    log::debug!("Flashing {} bytes to {}", data.len(), conf.address);
    write_memory(&mut port, conf.address, data)?;
    log::debug!("Flash Complete");
    sleep(Duration::from_millis(100));

    log::debug!("Resetting boot pin");
    gpio_boot.set_value(0)?;
    toggle_reset(&mut gpio_reset)?;

    log::info!("Done flashing");
    Ok(())
}

pub fn toggle_reset(gpio_reset: &mut GpioPin) -> Result<(), std::io::Error> {
    log::debug!("Toggling reset pin");
    gpio_reset.set_value(1)?;
    sleep(Duration::from_millis(100));
    gpio_reset.set_value(0)?;
    sleep(Duration::from_millis(100));
    Ok(())
}

pub fn connect_port(
    port_name: &str,
    baud_rate: u32,
) -> Result<Box<dyn serialport::SerialPort>, std::io::Error> {
    let s = SerialPortSettings {
        baud_rate,
        data_bits: DataBits::Eight,
        parity: Parity::Even,
        stop_bits: StopBits::One,
        flow_control: FlowControl::None,
        timeout: Duration::from_secs(1),
    };

    let mut port = serialport::open_with_settings(port_name, &s)?;

    let mut last_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "Failed to connect");
    for _ in 0..10 {
        if let Err(e) = crate::hello(&mut port) {
            last_err = e;
        } else {
            port.set_timeout(Duration::from_secs(20))?;
            return Ok(port);
        }
        sleep(Duration::from_millis(100));
    }
    Err(last_err)
}

pub enum GpioPin {
    None,
    Gpiod(LineHandle),
    Sysfs(u32),
}

fn cdev_error_to_io_error(e: gpio_cdev::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e))
}

impl GpioPin {
    pub fn new(pin: u32) -> Result<Self, std::io::Error> {
        // check if pin is already exported
        if std::path::Path::new(&format!("/sys/class/gpio/gpio{}", pin)).exists() {
            return Ok(GpioPin::Sysfs(pin));
        }
        let mut chip = Chip::new("/dev/gpiochip0").map_err(cdev_error_to_io_error)?;

        let handle = chip
            .get_line(pin)
            .map_err(cdev_error_to_io_error)?
            .request(LineRequestFlags::OUTPUT, 1, "stm32flash")
            .map_err(cdev_error_to_io_error)?;
        Ok(GpioPin::Gpiod(handle))
    }

    pub fn set_value(&mut self, value: u8) -> Result<(), std::io::Error> {
        match self {
            GpioPin::None => Ok(()),
            GpioPin::Gpiod(handle) => handle.set_value(value).map_err(cdev_error_to_io_error),
            GpioPin::Sysfs(pin) => Ok(std::fs::write(
                format!("/sys/class/gpio/gpio{}/value", pin),
                format!("{}", value),
            )?),
        }
    }
}
