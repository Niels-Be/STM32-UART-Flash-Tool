// https://www.st.com/resource/en/application_note/an3155-usart-protocol-used-in-the-stm32-bootloader-stmicroelectronics.pdf
use std::io::prelude::*;
use std::io::Error;
use std::io::ErrorKind;

const GET_COMMAND: [u8; 2] = [0x00, 0xFF];
const GET_VERSION_COMMAND: [u8; 2] = [0x01, 0xFE];
const GET_ID_COMMAND: [u8; 2] = [0x02, 0xFD];
const READ_MEMORY_COMMAND: [u8; 2] = [0x11, 0xEE];
const GO_COMMAND: [u8; 2] = [0x21, 0xDE];
const WRITE_MEMORY_COMMAND: [u8; 2] = [0x31, 0xCE];
const ERASE_MEMORY_COMMAND: [u8; 2] = [0x43, 0xBC];
const EXTENDED_ERASE_MEMORY_COMMAND: [u8; 2] = [0x44, 0xBB];

const ACK: u8 = 0x79;
#[allow(dead_code)]
const NACK: u8 = 0x1F;

const HELLO_BYTE: u8 = 0x7F;

pub fn hello<T: Read + Write>(port: &mut T) -> Result<(), Error> {
    // Send "Hello" byte
    port.write(&[HELLO_BYTE])?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Hello byte",
        ));
    }
    println!("got ack after hello byte");

    Ok(())
}

pub fn get<T: Read + Write>(port: &mut T) -> Result<Vec<u8>, Error> {
    // Send "Get" command
    port.write(&GET_COMMAND)?;

    println!("GET_COMMAND: {:?}", GET_COMMAND);

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Get command: ".to_string() + &response[0].to_string(),
        ));
    }

    println!("read");

    // Read number of bytes to follow
    port.read(&mut response)?;
    let num_bytes = response[0] as usize;

    println!("num_bytes: {}", num_bytes);

    // Read data bytes
    let mut data = vec![0; num_bytes];
    port.read_exact(&mut data)?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after data: ".to_string() + &response[0].to_string(),
        ));
    }

    Ok(data)
}

pub fn get_version<T: Read + Write>(port: &mut T) -> Result<(u8, Vec<u8>), Error> {
    // Send "Get Version" command
    port.write(&GET_VERSION_COMMAND)?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Get Version command: ".to_string()
                + &response[0].to_string(),
        ));
    }

    // Read version and supported commands
    let mut version_and_commands = [0; 3];
    port.read(&mut version_and_commands)?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after version and commands: ".to_string()
                + &response[0].to_string(),
        ));
    }

    let version = version_and_commands[0];
    let commands = version_and_commands[1..].to_vec();

    Ok((version, commands))
}

pub fn get_id<T: Read + Write>(port: &mut T) -> Result<u16, Error> {
    // Send "Get ID" command
    port.write(&GET_ID_COMMAND)?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Get ID command",
        ));
    }

    // Read number of bytes to follow
    port.read(&mut response)?;
    let _num_bytes = response[0] as usize;

    // Read product ID
    let mut id_bytes = [0; 2];
    port.read_exact(&mut id_bytes)?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after product ID: ".to_string() + &response[0].to_string(),
        ));
    }

    let id = u16::from_be_bytes(id_bytes);

    Ok(id)
}

pub fn read_memory<T: Read + Write>(
    port: &mut T,
    address: u32,
    num_bytes: u8,
) -> Result<Vec<u8>, Error> {
    // Send "Read Memory" command
    port.write(&READ_MEMORY_COMMAND)?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Read Memory command",
        ));
    }

    // Send address
    let address_bytes = address.to_be_bytes();
    let checksum = address_bytes.iter().fold(0xFF, |acc, &x| acc ^ x);
    port.write(&address_bytes)?;
    port.write(&[checksum])?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after address",
        ));
    }

    // Send number of bytes to read and checksum
    let checksum = num_bytes ^ 0xFF;
    port.write(&[num_bytes, checksum])?;

    // Wait for ACK and read data
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after number of bytes",
        ));
    }

    let mut data = vec![0; num_bytes as usize];
    port.read(&mut data)?;

    Ok(data)
}

pub fn go<T: Read + Write>(port: &mut T, address: u32) -> Result<(), Error> {
    // Send "Go" command
    port.write(&GO_COMMAND)?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Go command",
        ));
    }

    // Send address
    let address_bytes = address.to_be_bytes();
    let checksum = address_bytes.iter().fold(0xFF, |acc, &x| acc ^ x);
    port.write(&address_bytes)?;
    port.write(&[checksum])?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after address",
        ));
    }

    Ok(())
}

pub fn write_memory_block<T: Read + Write>(
    port: &mut T,
    address: u32,
    data: &[u8],
) -> Result<(), Error> {
    // Send "Write Memory" command
    port.write(&WRITE_MEMORY_COMMAND)?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Write Memory command",
        ));
    }

    // Send address
    let mut buf = Vec::with_capacity(5);
    buf.extend_from_slice(&address.to_be_bytes());
    let checksum = buf.iter().fold(0, |acc, &x| acc ^ x);
    buf.push(checksum);
    // println!("address: {:?}", buf);
    port.write(&buf)?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after address",
        ));
    }

    // Send number of bytes and data
    let length = (data.len() - 1) as u8; // Subtract 1 as per protocol
    let checksum = data.iter().fold(length, |acc, &x| acc ^ x);
    port.write(&[length])?;
    port.write(data)?;
    port.write(&[checksum])?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after data",
        ));
    }

    Ok(())
}

pub fn write_memory<T: Read + Write>(port: &mut T, address: u32, data: &[u8]) -> Result<(), Error> {
    let mut offset = 0;
    while offset < data.len() {
        let block_size = std::cmp::min(data.len() - offset, 256);
        println!("write to block: {:#x}", address + offset as u32);
        write_memory_block(
            port,
            address + offset as u32,
            &data[offset..offset + block_size],
        )?;
        offset += block_size;
    }
    Ok(())
}

pub fn erase_memory<T: Read + Write>(port: &mut T, sectors: &[u8]) -> Result<(), Error> {
    // Send "Erase Memory" command
    port.write(&ERASE_MEMORY_COMMAND)?;

    // Wait for ACK
    let mut response = [0; 1];
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Erase Memory command",
        ));
    }

    // Send number of sectors and sector numbers
    let length = (sectors.len() - 1) as u8; // Subtract 1 as per protocol
    let checksum = sectors.iter().fold(length, |acc, &x| acc ^ x);
    port.write(&[length])?;
    port.write(sectors)?;
    port.write(&[checksum])?;

    // Wait for ACK
    port.read(&mut response)?;
    if response[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after sectors",
        ));
    }

    Ok(())
}

pub fn erase_memory_global<T: Read + Write>(port: &mut T) -> Result<(), Error> {
    // Send the "Global Erase" command
    port.write(&ERASE_MEMORY_COMMAND)?;

    // Wait for ACK
    let mut ack: [u8; 1] = [0];
    port.read(&mut ack)?;

    if ack[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Erase Memory command",
        ));
    }

    // Send the number of pages to erase. 0xFF00 means global erase.
    const GLOBAL_ERASE_PAGES: [u8; 2] = [0xFF, 0x00];
    port.write(&GLOBAL_ERASE_PAGES)?;

    // Wait for ACK
    port.read(&mut ack)?;

    if ack[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after erase sectors",
        ));
    }

    Ok(())
}

pub fn extended_erase<T: Read + Write>(port: &mut T, pages: &[u16]) -> Result<(), Error> {
    // Command code for "Extended Erase" is 0x44
    port.write(&EXTENDED_ERASE_MEMORY_COMMAND)?;

    // Wait for ACK
    let mut ack: [u8; 1] = [0];
    port.read(&mut ack)?;

    if ack[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Extended Erase Memory command",
        ));
    }

    let mut bytes_to_send = Vec::<u8>::new();
    // Send the number of pages to erase
    bytes_to_send.extend_from_slice(&(pages.len() as u16).to_be_bytes());

    // Send the page numbers to erase
    for page in pages {
        bytes_to_send.extend_from_slice(&page.to_be_bytes());
    }
    // Calculate the checksum
    let checksum = bytes_to_send.iter().fold(0, |acc, &x| acc ^ x);
    bytes_to_send.push(checksum);

    port.write(&bytes_to_send)?;

    // Wait for ACK
    println!("wait for erase complete");
    port.read(&mut ack)?;

    if ack[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after erase sectors",
        ));
    }

    Ok(())
}

#[repr(u16)]
pub enum SpecialEraseType {
    MassErase = 0xFFFF,
    Bank1Erase = 0xFFFE,
    Bank2Erase = 0xFFFD,
}

pub fn extended_erase_special<T: Read + Write>(
    port: &mut T,
    cmd: SpecialEraseType,
) -> Result<(), Error> {
    // Send the "Extended Erase" command
    port.write(&EXTENDED_ERASE_MEMORY_COMMAND)?;

    // Wait for ACK
    let mut ack: [u8; 1] = [0];
    port.read(&mut ack)?;

    if ack[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after Extended Erase Memory command",
        ));
    }

    let mut bytes_to_send = Vec::<u8>::new();
    // Send the number of pages to erase
    bytes_to_send.extend_from_slice(&(cmd as u16).to_be_bytes());

    // Calculate the checksum
    let checksum = bytes_to_send.iter().fold(0, |acc, &x| acc ^ x);
    bytes_to_send.push(checksum);

    port.write(&bytes_to_send)?;

    // Wait for ACK
    println!("wait for erase complete");
    port.read(&mut ack)?;

    if ack[0] != ACK {
        return Err(Error::new(
            ErrorKind::Other,
            "Did not receive ACK after erase sectors",
        ));
    }

    Ok(())
}

pub fn flash_file<T: Read + Write>(port: &mut T, file: &str, address: u32) -> Result<(), Error> {
    let mut file = std::fs::File::open(file)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    const PAGE_SIZE: u32 = 0x800;
    let _num_pages = (data.len() as f32 / PAGE_SIZE as f32).ceil() as u8;
    let _page_offset = (address % PAGE_SIZE) as u8;
    // TODO: always erase block 0 and 1 ???
    extended_erase(port, &[0,1])?;

    write_memory(port, address, &data)?;

    Ok(())
}
