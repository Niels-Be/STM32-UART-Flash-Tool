STM32 USART Flash Tool
======================

An implementation of ST Micros USART Bootloader Protocol.  
Works for most STM32 Chips.

See:
https://www.st.com/resource/en/application_note/an3155-usart-protocol-used-in-the-stm32-bootloader-stmicroelectronics.pdf

### Usage

1. Set STM32 Chip into bootloader mode by toggeling boot0 and then reset
2. Flash Firmware in bin format
```
./stm32-firmware-loader-aarch64-android -p /dev/ttyXXXX flash ./usart_test.bin
```
3. Toggle boot0 back and reset again to run code

### Commands

```
USAGE:
    stm32-firmware-loader [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -b, --baudrate <BAUDRATE>    Sets the baudrate
    -h, --help                   Print help information
    -p, --port <PORT>            Sets the serial port to use
    -V, --version                Print version information

SUBCOMMANDS:
    erase_memory           
    erase_memory_global    
    flash                  
    get                    
    get_id                 
    get_version            
    go                     
    help
    read_memory            
    write_memory 
```