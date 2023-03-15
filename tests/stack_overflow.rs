#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use volatile::Volatile;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use MarOS::{exit_qemu, init, serial_println};
use MarOS::interrupts::init_idt;
use MarOS::QemuExitCode::Success;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("stack_overflow::stack_overflow...\t");
    MarOS::gdt::init();
    init_test_idt();
    stack_overflow();
    panic!("Execution continued after stack_overflow!")
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    MarOS::test_panic_handler(info)
}

#[allow(unconditional_recursion)]
fn stack_overflow(){
    stack_overflow();
    Volatile::new(0).read(); // avoid automatic optimization
}

lazy_static!{
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
            .set_handler_fn(test_double_fault_handler)
            .set_stack_index(MarOS::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
    ) -> ! {
    serial_println!("[ok]");
    exit_qemu(Success);
    loop {}
}