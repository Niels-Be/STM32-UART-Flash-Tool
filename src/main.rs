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
                .takes_value(true),
        )
        .arg(
            Arg::with_name("baudrate")
                .short('b')
                .long("baudrate")
                .value_name("BAUDRATE")
                .help("Sets the baudrate")
                .takes_value(true),
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
        .settings(&[clap::AppSettings::ArgRequiredElseHelp])
        .get_matches();

    let port_name = matches.value_of("port").unwrap_or("/dev/ttyUSB0");
    let baud_rate = matches
        .value_of("baudrate")
        .unwrap_or("115200")
        .parse()
        .unwrap();
    let s = SerialPortSettings {
        baud_rate,
        data_bits: DataBits::Eight,
        parity: Parity::Even,
        stop_bits: StopBits::One,
        flow_control: FlowControl::None,
        timeout: Duration::from_secs(1),
    };

    let mut port = serialport::open_with_settings(port_name, &s).unwrap();

    println!("Connecting to {}", port_name);
    for _ in 0..10 {
        port.write(&[0x7f]).unwrap();
        if let Ok(_) = hello(&mut port) {
            println!("Connected to {}", port_name);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
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
        }
        _ => (),
    }
}
