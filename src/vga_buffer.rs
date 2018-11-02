use spin::Mutex;

//Provide a global writer that can used as an interface from other modules
pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::Pink, Color::Black),
    buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
});

#[allow(dead_code)]         //Normally the compiler would issue a warning for each unused variant.
                            //By using the #[allow(dead_code)] attribute we disable these warnings for the Color enum.
#[repr(u8)]                 //each enum variant is stored as an u8
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

//Rust moves values by default instead of copying them like other languages.
//To fix it, we can implement the Copy trait for the ColorCode type.
//We also derive the Clone trait, since it's a requirement for Copy, and the Debug trait, which allows us to print this field for debugging purposes.
#[derive(Debug, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    //The value after the arrow is the return type
    //Const is an unchanable value
    const fn new(foreground: Color, background: Color) -> ColorCode {
        //The last row in the method is what is returned from the function.
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

//The repr(C) attribute guarantees that the struct's fields are laid out exactly like in a C struct and thus guarantees the correct field ordering.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

//Define buffer size
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

use volatile::Volatile;

struct Buffer {
    //Array of ScreenChars
    //Volatile tells the compiler that the write has side effects and should not be optimized away.
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

use core::ptr::Unique;

//The writer will always write to the last line and shift lines up when a line is full (or on \n).
pub struct Writer {
    //keeps track of the current position in the last row.
    column_position: usize,

    //specifies current foreground and background colors
    color_code: ColorCode,

    //Stores a pointer to the VGA buffer
    //Unique makes it possible to create a static Writer later
    buffer: Unique<Buffer>,
}

impl Writer {
    //method to write a single ASCII byte
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            //If the byte is the newline byte \n, the writer does not print anything. Instead it calls a new_line method
            b'\n' => self.new_line(),
            //In this match case, Other bytes get printed to the screen.
            byte => {
                //When printing a byte, the writer checks if the current line is full. In that case, a new_line call is required before to wrap the line.
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                //Row starts at 0, which is why we need to subtract 1
                let row = BUFFER_HEIGHT - 1;

                //Te column is the current position
                let col = self.column_position;

                let color_code = self.color_code;

                //write a new ScreenChar to the buffer at the current position.
                //Instead of a normal assignment using =, we're now using the write method. This guarantees that the compiler will never optimize away this write.
                self.buffer().chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code: color_code,
                });

                //increment current column position
                self.column_position += 1;
            }
        }
    }

    pub fn write_str(&mut self, s: &str) {
        //Loop through all bytes in the string and print them
        for byte in s.bytes() {
          self.write_byte(byte)
        }
    }

    //Convert the raw pointer in the buffer field into a safe mutable buffer reference.
    //The unsafe block is needed because the as_mut() method of Unique is unsafe.
    fn buffer(&mut self) -> &mut Buffer {
        unsafe{ self.buffer.as_mut() }
    }

    fn new_line(&mut self) {
        //Loop through buffer
        //we start the row at 1 as the 0th row is shifted off screen
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let buffer = self.buffer();
                let character = buffer.chars[row][col].read();

                //Move each character one row up
                buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT-1);

        //we are at the 0th position in the next row
        self.column_position = 0;
    }
    //This method clears a row by overwriting all of its characters with a space character.
    fn clear_row(&mut self, row: usize) {
        //Blank is a space character
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        //Loop through each position in the row and overwrite the characters with blank
        for col in 0..BUFFER_WIDTH {
            self.buffer().chars[row][col].write(blank);
        }
    }
}

//To support different types like integers or floats, we need to implement the core::fmt::Write trait
use core::fmt;

//The only required method of this trait is write_str that looks quite similar to our write_str method.
//To implement the trait, we just need to move it into an impl fmt::Write for Writer block and add a return type:
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
          self.write_byte(byte)
        }
        Ok(())
    }
}

pub fn print_something() {
    use core::fmt::Write;
    let mut writer = Writer {
        //Start writing at the beginning of the buffer
        column_position: 0,
        color_code: ColorCode::new(Color::Pink, Color::Black),
        buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
    };

    writer.write_byte(b'H');
    writer.write_str("ello! ");
    write!(writer, "The numbers are {} and {}", 42, 1.0/3.0);
}

//Prints to the VGA buffer. Says to use our static WRITER instead of _print
/*macro_rules! print {
    ($($arg:tt)*) => ({
        //Import the Write trait
        use core::fmt::Write;
        let mut writer = $crate::vga_buffer::WRITER.lock();

        //Instead of a _print function, we call the write_fmt method of our static Writer.
        //unwrap() panics if printing isn't successful
        writer.write_fmt(format_args!($($arg)*)).unwrap();
    });
}*/
//Deadlock free version.
//In order to fix the deadlock, we need to evaluate the arguments before locking the WRITER.
//We can do so by moving the locking and printing logic into a new print function.
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::vga_buffer::print(format_args!($($arg)*));
    });
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

macro_rules! println {
    //Both rules simply append a newline character (\n) to the format string and then invoke the print! macro
    //$fmt = format macro. used to print to the stdout
    //invocations with a single argument (e.g. println!("Hello"))
    ($fmt:expr) => (print!(concat!($fmt, "\n")));

    //invocations with additional parameters (e.g. println!("{}{}", 4, 2)).
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

/*macro_rules! print {
    //The macro expands to a call of the _print function in the io module.
    //The $crate variable ensures that the macro also works from outside the std crate.
    ($($arg:tt)*) => ($crate::io::_print(format_args!($($arg)*)));
}*/

pub fn clear_screen() {
    for _ in 0..BUFFER_HEIGHT {
        println!("");
    }
}
