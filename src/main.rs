use clap::{App, Arg, SubCommand};
use parse_int::parse;
use std::time::Duration;
use stm32_firmware_loader::helper::{connect_port, toggle_reset, GpioPin};
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
        .subcommand(SubCommand::with_name("erase_ext_all"))
        .subcommand(
            SubCommand::with_name("write_file")
                .arg(Arg::with_name("file").required(true))
                .arg(Arg::with_name("address").default_value("0x08000000")),
        )
        .subcommand(
            SubCommand::with_name("flash")
                .arg(Arg::with_name("file").required(true))
                .arg(Arg::with_name("address").default_value("0x08000000")),
        )
        .subcommand(SubCommand::with_name("reset"))
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
        if let Some(boot_pin) = boot_pin {
            if boot_pin != 0 {
                println!("Setting boot pin {}", boot_pin);
                let mut boot = GpioPin::new(boot_pin).expect("Failed to request boot pin");
                boot.set_value(1).expect("Failed to set boot pin");
                std::thread::sleep(Duration::from_millis(100));
                gpio_boot = Some(boot);
            }
        }
        if let Some(reset_pin) = reset_pin {
            if reset_pin != 0 {
                println!("Toggling reset pin {}", reset_pin);
                let mut reset = GpioPin::new(reset_pin).expect("Failed to request reset pin");
                std::thread::sleep(Duration::from_millis(100));
                toggle_reset(&mut reset).expect("Failed to toggle reset pin");
                gpio_reset = Some(reset);
            }
        }
    }

    if let Some("reset") = matches.subcommand_name() {
        if let Some(gpio_boot) = &mut gpio_boot {
            println!("Resetting boot pin");
            gpio_boot.set_value(0).expect("Failed to reset boot pin");
        }

        toggle_reset_opt(&mut gpio_reset);
        return;
    }

    println!("Connecting on {} {}", port_name, baud_rate);
    let mut port = connect_port(port_name, baud_rate).expect("Failed to connect");
    println!("Connected on {}", port_name);

    match matches.subcommand() {
        Some(("get", _)) => {
            let res = get(&mut port);
            println!("Get: {:?}", res);
        }
        Some(("get_version", _)) => {
            let res = get_version(&mut port);
            println!("Version: {:?}", res);
        }
        Some(("get_id", _)) => {
            let res = get_id(&mut port);
            println!("ID: {:?}", res);
        }
        Some(("read_memory", sub_m)) => {
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let size = sub_m.value_of("size").unwrap().parse().unwrap();
            let res = read_memory_vec(&mut port, address, size);
            println!("Memory: {:?}", res);
        }
        Some(("go", sub_m)) => {
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let res = go(&mut port, address);
            println!("Go: {:?}", res);
        }
        Some(("write_memory", sub_m)) => {
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let data = sub_m.value_of("data").unwrap().as_bytes().to_vec();
            let res = write_memory(&mut port, address, &data);
            println!("Write: {:?}", res);
        }
        Some(("erase_memory", sub_m)) => {
            let page: u8 = sub_m.value_of("page").unwrap().parse().unwrap();
            let count: u8 = sub_m.value_of("count").unwrap().parse().unwrap();
            let res = erase_memory(&mut port, &(page..page + count).collect::<Vec<u8>>());
            println!("Erase: {:?}", res);
        }
        Some(("erase_memory_global", _)) => {
            let res = erase_memory_global(&mut port);
            println!("Erase global: {:?}", res);
        }
        Some(("erase_ext_all", _)) => {
            let res = extended_erase_special(&mut port, SpecialEraseType::MassErase);
            println!("Erase ext all: {:?}", res);
        }
        Some(("write_file", sub_m)) => {
            let file = sub_m.value_of("file").unwrap();
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();
            let res = flash_file(&mut port, file, address);
            println!("Flash: {:?}", res);
        }
        Some(("flash", sub_m)) => {
            let file = sub_m.value_of("file").unwrap();
            let address = parse(sub_m.value_of("address").unwrap()).unwrap();

            // Note: this might time out for some reason, it does succeed anyway
            let res = extended_erase_special(&mut port, SpecialEraseType::MassErase);
            if let Err(e) = res {
                println!("Reconnect after erase: {:?}", e);
                // close current port
                drop(port);

                toggle_reset_opt(&mut gpio_reset);
                port = connect_port(port_name, baud_rate).expect("Failed to connect");
            }

            println!("Flashing {} at {}", file, address);
            let res = flash_file(&mut port, file, address);
            println!("Flash: {:?}", res);
        }
        Some(("reset", _)) => {
            // nothing to do
        }
        _ => (),
    }

    if let Some(gpio_boot) = &mut gpio_boot {
        println!("Resetting boot pin");
        gpio_boot.set_value(0).expect("Failed to reset boot pin");
    }

    toggle_reset_opt(&mut gpio_reset);
}

fn toggle_reset_opt(gpio_reset: &mut Option<GpioPin>) {
    if let Some(gpio_reset) = gpio_reset {
        toggle_reset(gpio_reset).expect("Failed to toggle reset pin");
    }
}
