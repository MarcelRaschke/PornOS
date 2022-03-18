#![no_std]
#![no_main]
#![feature(const_ptr_offset)]
#![feature(custom_test_frameworks)]
#![test_runner(pornos::porno_test)]
#![reexport_test_harness_main = "test_main"]

use pornos;
use pornos::println;
use stivale_boot::v2::{StivaleAnyVideoTag, StivaleHeader};

#[no_mangle]
pub extern "C" fn pornos_entry() -> ! {
    println!("Starting the OS...");

    #[cfg(test)]
    test_main();

    println!("Entering loop...");
    loop {}
}

pub const PORNOS_STACK_SIZE: usize = 8_192;
pub static PORNOS_STACK: [u8; PORNOS_STACK_SIZE] = [0; PORNOS_STACK_SIZE];

#[used]
#[no_mangle]
#[link_section = ".stivale2hdr"]
pub static STIVALE_HEADER: StivaleHeader = StivaleHeader::new()
    .stack(unsafe { PORNOS_STACK.as_ptr().offset(PORNOS_STACK_SIZE as isize) })
    .flags(0b11110)
    .tags(&ANY_VIDEO_HEADER_TAG as *const StivaleAnyVideoTag as *const ());

pub static ANY_VIDEO_HEADER_TAG: StivaleAnyVideoTag = StivaleAnyVideoTag::new().preference(1);
