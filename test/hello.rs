#![feature(lang_items)]
#![feature(start)]
#![allow(internal_features)]
#![no_std]
#![feature(core_intrinsics)]
use core::panic::PanicInfo;
#[lang = "eh_personality"]
fn rust_eh_personality() {}
extern "C" {
    fn puts(string:*const u8);
    fn malloc(size:usize)->*mut ();
    fn free(ptr:*const ());
}
#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
#[start]
fn main(_argc: isize, _argv: *const *const u8)->isize{
    unsafe{
        let message:*mut u8 = malloc(16) as *mut u8;
        *message.offset(0) = 0x48;
        *message.offset(1) =  0x65;
        *message.offset(2) =  0x6C;          
        *message.offset(3) =  0x6C;
        *message.offset(4) =  0x6F;
        *message.offset(5) =  0x20;
        *message.offset(6) =  0x66;
        *message.offset(7) =  0x72;
        *message.offset(8) =  0x6F;
        *message.offset(9) =  0x6D;
        *message.offset(10) =  0x20;
        *message.offset(11) =  0x43;
        *message.offset(12) =  0x4C;
        *message.offset(13) =  0x52;
        *message.offset(14) =  0x21;
        *message.offset(15) =  0x00;
        puts(message);
        free(message as *const _ as *const _);
    }
    0
}