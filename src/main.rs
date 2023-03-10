#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(MarOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo; 
use MarOS::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("MarOS");


    MarOS::init();

    x86_64::instructions::interrupts::int3(); // generates a breakpoint

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}",info);
    loop{}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    MarOS::test_panic_handler(info)
}


#[test_case]
fn trivial_assertion(){
    assert_eq!(3,3);
}

