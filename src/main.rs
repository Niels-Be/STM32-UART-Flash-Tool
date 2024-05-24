use clap::{App, Arg, SubCommand};
use parse_int::parse;
use serialport::prelude::*;
use std::time::Duration;
use stm32_firmware_loader::*;

fn main() {
    let matches = App::new("STM32 Bootloader Utility")
        .version("1.0")
        .author("KBST GmbH <info@kbst-gmbh.de>")
        .about("Interacts with STM32 bootloader")
        .arg(
            Arg::with_name("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the serial port to use")
                .takes_value(true)
                .default_value("/dev/ttyHS1"),
        )
        .arg(
            Arg::with_name("baudrate")
                .short('b')
                .long("baudrate")
                .value_name("BAUDRATE")
                .help("Sets the baudrate")
                .takes_value(true)
                .default_value("115200"),
        )
        .arg(
            Arg::with_name("boot-pin")
                .short('B')
                .long("boot-pin")
                .value_name("BOOT_PIN")
                .help("Toggles the boot gpio pin. 0 to disable")
                .takes_value(true)
                .default_value("9"),
        )
        .arg(
            Arg::with_name("reset-pin")
                .short('R')
                .long("reset-pin")
                .value_name("RESET_PIN")
                .help("Toggles the reset gpio pin. 0 to disable")
                .takes_value(true)
                .default_value("8"),
        )
        .subcommand(SubCommand::with_name("get"))
        .subcommand(SubCommand::with_name("get_version"))
        .subcommand(SubCommand::with_name("get_id"))
        .subcommand(
            SubCommand::with_name("read_memory")
                .arg(Arg::with_name("address").required(true))
                .arg(Arg::with_name("size").required(true)),
        )
        .subcommand(
            SubCommand::with_name("go").arg(Arg::with_name("address").default_value("0x08000000")),
        )
        .subcommand(
            SubCommand::with_name("write_memory")
                .arg(Arg::with_name("address").required(true))
                .arg(Arg::with_name("data").required(true)),
        )
        .subcommand(
            SubCommand::with_name("erase_memory")
                .arg(Arg::with_name("page").required(true))
                .arg(Arg::with_name("count").required(true)),
        )
        .subcommand(SubCommand::with_name("erase_memory_global"))
        .subcommand(
            SubCommand::with_name("flash")
                .arg(Arg::with_name("file").required(true))
                .arg(Arg::with_name("address").default_value("0x08000000")),
        )
        .settings(&[
            clap::AppSettings::ArgRequiredElseHelp,
            clap::AppSettings::SubcommandRequiredElseHelp,
        ])
        .get_matches();

    let port_name = matches.value_of("port").expect("missing port");
    let baud_rate = matches
        .value_of("baudrate")
        .expect("missing baudrate")
        .parse()
        .expect("invalid baudrate");
    let boot_pin: Option<u32> = matches
        .value_of("boot-pin")
        .map(|x| x.parse().expect("invalid boot pin"));
    let reset_pin: Option<u32> = matches
        .value_of("reset-pin")
        .map(|x| x.parse().expect("invalid reset pin"));

    let mut gpio_boot = None;
    let mut gpio_reset = None;
    if boot_pin.is_some() || reset_pin.is_some() {
        use gpio_cdev::Chip;
        use gpio_cdev::LineRequestFlags;
        let mut chip = Chip::new("/dev/gpiochip0").expect("Failed to open gpio chip");
        // println!("Detected {} lines", chip.num_lines());

        if let Some(boot_pin) = boot_pin {
            if boot_pin != 0 {
                println!("Setting boot pin {}", boot_pin);
                let boot = chip
                    .get_line(boot_pin)
                    .expect("Failed to request boot pin")
                    .request(LineRequestFlags::OUTPUT, 1, "stm32flash")
                    .expect("Failed to request boot pin");
                boot.set_value(1).expect("Failed to set boot pin");
                std::thread::sleep(Duration::from_millis(100));
                gpio_boot = Some(boot);
            }
        }
        if let Some(reset_pin) = reset_pin {
            if reset_pin != 0 {
                println!("Toggling reset pin {}", reset_pin);
                let reset = chip
                    .get_line(reset_pin)
                    .expect("Failed to request reset pin")
                    .request(LineRequestFlags::OUTPUT, 0, "stm32flash")
                    .expect("Failed to request reset pin");
                std::thread::sleep(Duration::from_millis(100));
                reset.set_value(1).expect("Failed to set reset pin");
                std::thread::sleep(Duration::from_millis(100));
                reset.set_value(0).expect("Failed to reset reset pin");
                std::thread::sleep(Duration::from_millis(100));
                gpio_reset = Some(reset);
            }
        }
    }

    println!("Connecting on {} {}", port_name, baud_rate);
    let mut port = connect_port(port_name, baud_rate).expect("Failed to connect");
    println!("Connected on {}", port_name);

    if let Some(gpio_boot) = gpio_boot {
        println!("Resetting boot pin");
        gpio_boot.set_value(0).expect("Failed to reset boot pin");
    }

    match matches.subcommand() {
        Some(("get", _)) => {
            let res = get(&mut port).unwrap();
            println!("Get: {:?}", res);
        }
        Some(("get_version", _)) => {
            let res = get_version(&mut port).unwrap();
            println!("Version: {:?}", res);
        }
        Some(("get_id", _)) => {
            let res = get_id(&mut port).unwrap();
            println!("ID: {:?}", res);
        }
        Some(("read_memory", sub_m)) => {
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let size = sub_m.value_of("size").unwrap().parse().unwrap();
            let res = read_memory(&mut port, address, size).unwrap();
            println!("Memory: {:?}", res);
        }
        Some(("go", sub_m)) => {
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let res = go(&mut port, address).unwrap();
            println!("Go: {:?}", res);
        }
        Some(("write_memory", sub_m)) => {
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let data = sub_m.value_of("data").unwrap().as_bytes().to_vec();
            let res = write_memory(&mut port, address, &data).unwrap();
            println!("Write: {:?}", res);
        }
        Some(("erase_memory", sub_m)) => {
            let page: u8 = sub_m.value_of("page").unwrap().parse().unwrap();
            let count: u8 = sub_m.value_of("count").unwrap().parse().unwrap();
            let res = erase_memory(&mut port, &(page..page + count).collect::<Vec<u8>>()).unwrap();
            println!("Erase: {:?}", res);
        }
        Some(("erase_memory_global", _)) => {
            let res = erase_memory_global(&mut port).unwrap();
            println!("Erase global: {:?}", res);
        }
        Some(("flash", sub_m)) => {
            let file = sub_m.value_of("file").unwrap();
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let res = flash_file(&mut port, file, address).unwrap();
            println!("Flash: {:?}", res);

            if let Some(gpio_reset) = gpio_reset {
                println!("Toggling reset pin");
                gpio_reset.set_value(1).expect("Failed to set reset pin");
                std::thread::sleep(Duration::from_millis(100));
                gpio_reset.set_value(0).expect("Failed to reset reset pin");
            }
        }
        _ => (),
    }
}

fn connect_port(port_name: &str, baud_rate: u32) -> Result<Box<dyn SerialPort>, std::io::Error> {
    let s = SerialPortSettings {
        baud_rate,
        data_bits: DataBits::Eight,
        parity: Parity::Even,
        stop_bits: StopBits::One,
        flow_control: FlowControl::None,
        timeout: Duration::from_secs(1),
    };

    let mut port = serialport::open_with_settings(port_name, &s).expect("Failed to open port");

    let mut last_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "Failed to connect");
    for _ in 0..10 {
        if let Err(e) = hello(&mut port) {
            last_err = e;
        } else {
            port.set_timeout(Duration::from_secs(30))?;
            return Ok(port);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(last_err)
}
