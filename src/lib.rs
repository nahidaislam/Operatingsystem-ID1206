// root source file

//#! = define attribute
#![feature(lang_items)]     //turn on "defining language item" feature
#![feature(const_fn)]       //be able to use const functionsrc
#![feature(unique)]
#![feature(allocator_api)]
#![feature(const_atomic_usize_new)]
#![feature(global_allocator)]
#![feature(alloc)]
#![no_std]                  //prevent automatic linking of standard library

#[macro_use]
extern crate alloc;

extern crate rlibc;
extern crate volatile;
extern crate spin;
extern crate multiboot2;    //gives info about mapped kernel section and available memory

#[macro_use]
extern crate bitflags;
extern crate x86_64;

#[macro_use]
extern crate once;
extern crate linked_list_allocator;

#[macro_use]
mod vga_buffer;
mod memory;


/*old main
#[no_mangle] // disable name mangling that rust uses to get unique function names
pub extern fn rust_main(multiboot_information_address: usize) { //function name must stay as it is
    //use core::fmt::Write;
    //vga_buffer::WRITER.lock().write_str("Hello again");
    //write!(vga_buffer::WRITER.lock(), ", some numbers: {} {}", 42, 1.337);

    use memory::FrameAllocator;

    vga_buffer::clear_screen();
    //println!("{}", { println!("inner"); "outer" });
    println!("Hello World{}", "!");

    // load the multiboot information address
    // get the memory map tag that contains a list of all available RAM areas
    let boot_info = unsafe{ multiboot2::load(multiboot_information_address) };
    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required"); // provide panic (error) messages, if we cannot find memory_map_tag

    // print all available memory areas
    println!("memory areas:");
    for area in memory_map_tag.memory_areas() {
        println!("    start: 0x{:x}, length: 0x{:x}",
            area.base_addr, area.length);
    }

    // print sections of kernel ELF file
    let elf_sections_tag = boot_info.elf_sections_tag()
    .expect("Elf-sections tag required");
    // print the start address and size of all kernel sections
    // If section is writable the 0x1 bit is set in flags
    // 0x4 marks an executable section and 0x2 indicates that the section was loaded in memory
    println!("kernel sections:");
    for section in elf_sections_tag.sections() {
        println!("    addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
            section.addr, section.size, section.flags);
    }

    // calculate start and end address of the loaded kernel
    let kernel_start = elf_sections_tag.sections().map(|s| s.addr)
    .min().unwrap();
    let kernel_end = elf_sections_tag.sections().map(|s| s.addr + s.size)
    .max().unwrap();

    // the other used memory area is the Multtiboot information structure
    let multiboot_start = multiboot_information_address;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);
    println!("Kernel start: 0x{:x}, Kernel end: 0x{:x}", kernel_start, kernel_end);
    println!("Multiboot start: 0x{:x}, Multiboot end: 0x{:x}", multiboot_start, multiboot_end);

    // create a frame allocator
    let mut frame_allocator = memory::AreaFrameAllocator::new(
    kernel_start as usize, kernel_end as usize, multiboot_start,
    multiboot_end, memory_map_tag.memory_areas());

/*    //memory::test_paging(&mut frame_allocator);
    println!("{:?}", frame_allocator.allocate_frame());
    // allocates all frames and prints out the total number of allocated frames
    for i in 0.. {
        // continues until there are no free frames left, allocate_frame() returns None
        if let None = frame_allocator.allocate_frame() {
            println!("allocated {} frames", i);
            break;
        }
    }*/

    enable_nxe_bit();
    enable_write_protect_bit();
    memory::remap_the_kernel(&mut frame_allocator, boot_info);
    frame_allocator.allocate_frame(); // new: try to allocate a frame
    println!("It did not crash!");

    use alloc::boxed::Box;
    let heap_test = Box::new(42);

    println!("Heap test: {}", heap_test);

    loop{}
}*/

#[no_mangle]
pub extern "C" fn rust_main(multiboot_information_address: usize) {
    // ATTENTION: we have a very small stack and no guard page
    vga_buffer::clear_screen();
    println!("Hello World{}", "!");

    // load the multiboot information address
    let boot_info = unsafe {
        multiboot2::load(multiboot_information_address)
    };
    enable_nxe_bit();
    enable_write_protect_bit();

    // set up guard page and map the heap pages
    memory::init(boot_info);

    unsafe {
    HEAP_ALLOCATOR.lock().init(HEAP_START, HEAP_START + HEAP_SIZE);
    }

    use alloc::boxed::Box;
    let mut heap_test = Box::new(42);
    *heap_test -= 15;
    let heap_test2 = Box::new("hello");
    println!("{:?} {:?}", heap_test, heap_test2);

    let mut vec_test = vec![1,2,3,4,5,6,7];
    vec_test[3] = 42;
    for i in &vec_test {
        print!("{} ", i);
    }

    for i in 0..10000 {
    format!("Some String");
    }

    println!("It did not crash!");

    loop {}
}

//enable NXE bit
fn enable_nxe_bit() {
    use x86_64::registers::msr::{IA32_EFER, rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

//enable write protection for kernel
fn enable_write_protect_bit() {
    use x86_64::registers::control_regs::{cr0, cr0_write, Cr0};

    unsafe { cr0_write(cr0() | Cr0::WRITE_PROTECT) };
}

// panic handler, prints PANIC when something goes wrong
// shows which file and line the error occurred in
#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str,
    line: u32) -> !
{
    println!("\n\nPANIC in {} at line {}:", file, line);
    println!("    {}", fmt);
    loop{}
}


// define that these functions are our lagnuage items
// if something goes wrong and cannot reasonably be handled, the thread panics.
#[lang = "eh_personality"] extern fn eh_personality() {}       //used for Rust unwinding on panic!
//#[lang = "panic_fmt"] #[no_mangle] pub extern fn panic_fmt() -> ! {loop{}}      //doesn't return (required by ! return type), put in loop

use memory::heap_allocator::BumpAllocator;
use linked_list_allocator::LockedHeap;

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
//#[global_allocator]
//static HEAP_ALLOCATOR: BumpAllocator = BumpAllocator::new(HEAP_START, HEAP_START + HEAP_SIZE);
