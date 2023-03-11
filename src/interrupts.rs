use core::intrinsics::breakpoint;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init_idt() {
    unsafe {
        IDT.breakpoint.set_handler_fn();
        IDT.load();
    }
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame : InterruptStackFrame,
){
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
