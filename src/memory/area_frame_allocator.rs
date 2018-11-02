// the frame allocator

use memory::{Frame, FrameAllocator};
use multiboot2::{MemoryAreaIter, MemoryArea};

pub struct AreaFrameAllocator {
    next_free_frame: Frame,     // counter that is increased every time we return a frame
    current_area: Option<&'static MemoryArea>,  //holds the memory area that next_free_frame points to
    areas: MemoryAreaIter,  //if next_free_frame leaves current_area we look at the next one in areas

    // used to avoid returning already used fields
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

//allocate and deallocate a frame
impl FrameAllocator for AreaFrameAllocator {

    fn allocate_frame(&mut self) -> Option<Frame> {

        // Some: returns value if it exist, otherwise None
        // put self.current_area (a memory area) in area, if the area exist continue
        // if area does not exist do nothing -> no free frames left
        if let Some(area) = self.current_area {

        // "Clone" the frame to return it if it's free. Frame doesn't
        // implement Clone, but we can construct an identical frame.
        // put the frame number to the number in our Frame struct
        // save the frame number of the next_free_frame
        let frame = Frame{ number: self.next_free_frame.number };

        // the last frame of the current area
        let current_area_last_frame = {
            // get the last address in the memory area (the last frame)
            let address = area.base_addr + area.length - 1;
            // get the frame of the physical address
            Frame::containing_address(address as usize)
        };

        // if our frame number is larger than the last frame of current area
        // our frame does not fit in the current area
        if frame > current_area_last_frame {
            // all frames of current area are used, switch to next area
            self.choose_next_area();
        }
        // `frame` is used by the kernel
        else if frame >= self.kernel_start && frame <= self.kernel_end {
            self.next_free_frame = Frame {
                // take the next frame that lies after the kernel ends
                number: self.kernel_end.number + 1
            };
        }
        // `frame` is used by the multiboot information structure
        else if frame >= self.multiboot_start && frame <= self.multiboot_end {
            self.next_free_frame = Frame {
                // take the next frame that lies after the multiboot ends
                number: self.multiboot_end.number + 1
            };
        }
        // frame is unused, increment `next_free_frame` and return it
        else {
            self.next_free_frame.number += 1;
            return Some(frame);
        }
        // `frame` was not valid, try it again with the updated `next_free_frame`
        self.allocate_frame()
    }
    else {
        None // no free frames left
    }
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        // TODO (see below)
    }
}

// choose area that contains the minimal base address with free frames 
impl AreaFrameAllocator {

    // make allocator unstable
    pub fn new(kernel_start: usize, kernel_end: usize,
      multiboot_start: usize, multiboot_end: usize,
      memory_areas: MemoryAreaIter) -> AreaFrameAllocator
    {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
        };
        allocator.choose_next_area();
        allocator
    }

    // chooses the area with the minimal base address that still has free frames
    // next_free_frame is smaller than its last frame
    fn choose_next_area(&mut self) {

    self.current_area = self.areas.clone().filter(|area| {

        // check if we have any free frames in the area
        // if the frame of the last address is equal or bigger than next_free_frame,
        // we know that the frame is free since everything less than next_free_frame is not free
        let address = area.base_addr + area.length - 1;
        Frame::containing_address(address as usize) >= self.next_free_frame
        // returns the element that gives the minimum value from the specified function
    }).min_by_key(|area| area.base_addr);

    // if next_free_frame is below the minimal address with free frames
    if let Some(area) = self.current_area {
        let start_frame = Frame::containing_address(area.base_addr as usize);
        if self.next_free_frame < start_frame {
            self.next_free_frame = start_frame;
        }
    }
}
}
