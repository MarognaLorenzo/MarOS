#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(MarOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use MarOS::println;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[test_case]
fn test_println() {
    println!("println test from basic_boot");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    MarOS::test_panic_handler(info)
}

