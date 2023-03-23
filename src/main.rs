#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(MarOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo; 
use MarOS::{hlt_loop, println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("MarOS");


    MarOS::init();

    fn stack_overflow(){
        stack_overflow()
    }

    // stack_overflow();

    #[cfg(test)]
    test_main();

    hlt_loop()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}",info);
    hlt_loop()
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

