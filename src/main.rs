#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(MarOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{Page, Translate};
use x86_64::VirtAddr;
use MarOS::{hlt_loop, memory, println};
use MarOS::memory::BootInfoFrameAllocator;

entry_point!(kernel_main);

 fn kernel_main(boot_info: &'static BootInfo) -> ! {
     println!("MarOS");
     MarOS::init();


     let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

     let mut mapper = unsafe { memory::init(phys_mem_offset)};
     let mut frame_allocator = unsafe {
         BootInfoFrameAllocator::init(&boot_info.memory_map)
     };

     let page = Page::containing_address(VirtAddr::new(0));
     memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

     let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
     unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};



     let addresses = [
         // the identity-mapped vga buffer page
         0xb8000,
         // some code page
         0x201008,
         // some stack page
         0x0100_0020_1a10,
         // virtual address mapped to physical address 0
         boot_info.physical_memory_offset,
     ];

     for &address in &addresses {
         let virt = VirtAddr::new(address);
         let phys = mapper.translate_addr(virt);
         println!("{:?} -> {:?}", virt, phys);
     }

     #[cfg(test)]
     test_main();

     hlt_loop()
 }

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    MarOS::test_panic_handler(info)
}


#[test_case]
fn trivial_assertion() {
    assert_eq!(3, 3);
}

