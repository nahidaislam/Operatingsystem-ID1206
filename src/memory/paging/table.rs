// model the page table

use core::marker::PhantomData;      // needed since unused type parameters are not allowed in Rust
use memory::paging::entry::*;
use memory::paging::ENTRY_COUNT;
use memory::FrameAllocator;
use core::ops::{Index, IndexMut};


// P4 table is available at 0xfffffffffffff000
pub const P4: *mut Table<Level4> = 0xffffffff_fffff000 as *mut _;

pub struct Table<L: TableLevel> {
    // array of 512 entries
    // Entry - what it contains
    // ENTRY_COUNT - size of array
    entries: [Entry; ENTRY_COUNT],
    level: PhantomData<L>,          // indicate that your struct owns data of type L
}


impl<L> Index<usize> for Table<L> where L: TableLevel {
    type Output = Entry;

    // takes an index and returns the entry on that index
    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L> where L: TableLevel {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

impl<L> Table<L> where L: TableLevel {

    // sets all entries to unused
    // needed when we create a new page table
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L> Table<L> where L: HierarchicalLevel {

    // convert addresses into references
    // convert addresses to raw pointers throu as
    // convert addresses to Rust references through mut
    // return the table of the next level (P3 for P4, P1 for P2 and so on..)
    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        //if address at place index exists -> make it into a reference
        self.next_table_address(index).map(|address| unsafe { &*(address as *const _) })
    }
    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index).map(|address| unsafe { &mut *(address as *mut _) })
    }

    // calculate the next page table address
    fn next_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].flags();
        // next table address is only valid if the corresponding entry is present and does not create a huge page
        if entry_flags.contains(PRESENT) && !entry_flags.contains(HUGE_PAGE) {
            let table_address = self as *const _ as usize;
            // formula to calculate next address, the address of next page table
            Some((table_address << 9) | (index << 12))
        } else {
            None
        }
    }

    // return next table if it exists or create a new one
    pub fn next_table_create<A>(&mut self, index: usize, allocator: &mut A) -> &mut Table<L::NextLevel>
    where A: FrameAllocator
{
    // if there does not exist a next table
    if self.next_table(index).is_none() {
        assert!(!self.entries[index].flags().contains(HUGE_PAGE),
                "mapping code does not support huge pages");
        // allocate frames
        let frame = allocator.allocate_frame().expect("no frames available");
        // set the present and writeable bits
        self.entries[index].set(frame, PRESENT | WRITABLE);
        // set all entries to unused
        self.next_table_mut(index).unwrap().zero();
    }
    self.next_table_mut(index).unwrap()
    }
}


// model the different tables with traits and empty enums
// empty enum has size 0 and disappears after compiling
pub trait TableLevel {}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

// to differentiate the P1 table from the other tables, use HierarchicalLevel trait
// we should only be able to use next_table methods on P4, P3 and P2
// not on P1 since it doesn't have a next table
// define the next levels for each of the tables
pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

impl HierarchicalLevel for Level4 {
    type NextLevel = Level3;
}

impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}

impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}
