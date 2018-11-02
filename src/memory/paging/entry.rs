// model page table entries

use memory::Frame; // needed later
use multiboot2::ElfSection;

pub struct Entry(u64);

//convert elf section flags to page table flags
impl EntryFlags {

    pub fn from_elf_section_flags(section: &ElfSection) -> EntryFlags {
        use multiboot2::{ELF_SECTION_ALLOCATED, ELF_SECTION_WRITABLE,
            ELF_SECTION_EXECUTABLE};

        let mut flags = EntryFlags::empty();

        if section.flags().contains(ELF_SECTION_ALLOCATED) {
            // section is loaded to memory
            flags = flags | PRESENT;
        }
        if section.flags().contains(ELF_SECTION_WRITABLE) {
            flags = flags | WRITABLE;
        }
        if !section.flags().contains(ELF_SECTION_EXECUTABLE) {
            flags = flags | NO_EXECUTE;
        }

        flags
    }
}

impl Entry {

    //check if entry is unused
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    //set entry to unused
    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    // extract flags from entry
    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)  // drop bits that do not correspond to flag
    }

    // extract physical address
    pub fn pointed_frame(&self) -> Option<Frame> {
        //if entry is present
        if self.flags().contains(PRESENT) {
            // return corresponding frame
            Some(Frame::containing_address(
                self.0 as usize & 0x000fffff_fffff000   //mask bits 12-51 which is the physical address
            ))
        } else {
            None
        }
    }
    // modify entries
    // update flags
    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        // check if entry is valid
        assert!(frame.start_address() & !0x000fffff_fffff000 == 0);
        // sets the needed flags from the start address
        self.0 = (frame.start_address() as u64) | flags.bits();
    }
}

// flags of the physical address
bitflags! {
    pub struct EntryFlags: u64 {
        const PRESENT =         1 << 0;     //page is in memory
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}
