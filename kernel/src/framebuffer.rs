#![no_std]
#![feature(abi_x86_interrupt)]

use core::fmt;
use bootloader_api::info::{ FrameBufferInfo, PixelFormat};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use lazy_static::lazy_static;
use x86_64::instructions::interrupts::without_interrupts;
use crate::{ serial_println};
use spin::Mutex;

const SCALE: usize = 1;
const SIZE : usize = 8;
const TOTAL : usize = SIZE * SCALE;
#[derive(Debug, Clone, Copy)]
pub struct Rgba
{
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

pub struct Writer
{
    row_pos: usize,
    column_pos: usize,
    rbga: Rgba,
    info: FrameBufferInfo,
    frame_buffer: &'static mut [u8],
    cursor_visible : bool,
}


impl Rgba
{
    pub fn new(color_code: u32) -> Self
    {
        Rgba
        {
            red: (color_code >> 24 & 0xFF) as u8,
            green: (color_code >> 16 & 0xFF) as u8,
            blue: (color_code >> 8 & 0xFF) as u8,
            alpha: (color_code & 0xFF) as u8,
        }
    }
}
impl Writer
{
    pub fn new(buffer: &'static mut [u8], info: FrameBufferInfo, color: u32) -> Self
    {
        Writer {
            info,
            rbga: Rgba::new(color),
            row_pos: 0,
            column_pos: 0,
            frame_buffer: buffer,
            cursor_visible: false,

        }
    }
    fn draw_pixel(&mut self, pos_x: usize, pos_y: usize)
    {
        let byte_index = (pos_y * self.info.stride + pos_x) * self.info.bytes_per_pixel;
        match self.info.pixel_format {
            PixelFormat::Rgb =>
                {
                    self.frame_buffer[byte_index] = self.rbga.red;
                    self.frame_buffer[byte_index + 1] = self.rbga.green;
                    self.frame_buffer[byte_index + 2] = self.rbga.blue;
                }
            PixelFormat::Bgr =>
                {
                    self.frame_buffer[byte_index] = self.rbga.blue;
                    self.frame_buffer[byte_index + 1] = self.rbga.green;
                    self.frame_buffer[byte_index + 2] = self.rbga.red;
                }
            _ =>
                {
                    serial_println!("Unsupported pixel format!, what kind of hardware is it?")
                }
        }
    }

    fn new_line(&mut self)
    {
        self.column_pos = 0;
        self.row_pos += TOTAL;
        if self.row_pos + TOTAL > self.info.height {
            self.move_up();

            self.row_pos -= TOTAL;
        }
    }
    fn move_up(&mut self) {
        let bytes_per_pixel_row = self.info.stride * self.info.bytes_per_pixel;
        let bytes_per_text_row = bytes_per_pixel_row * TOTAL;


        self.frame_buffer.copy_within(bytes_per_text_row.., 0);

        self.clear_line();
    }

    fn clear_line(&mut self) {
        let bytes_per_pixel_row = self.info.stride * self.info.bytes_per_pixel;
        let bytes_per_text_row = bytes_per_pixel_row * TOTAL;

        let start_of_last_line = self.frame_buffer.len() - bytes_per_text_row;

        self.frame_buffer[start_of_last_line..].fill(0);
    }


    fn write_char(&mut self, c: char)
    {
        if c == '\u{8}' {
            self.delete_char();
            return;
        }
        self.clear_cursor();
        if c == '\n' {
            self.new_line();
            self.draw_cursor();
            return;
        }
        if self.column_pos + TOTAL > self.info.width {
            self.new_line();
        }
        if let Some(glyph) = BASIC_FONTS.get(c)
        {
            for x_offset in 0..SIZE
            {
                for y_offset in 0..SIZE
                {
                    if glyph[y_offset] & (1 << x_offset) != 0
                    {
                        for sx in 0..SCALE {
                            for sy in 0..SCALE {
                                let absolute_x = self.column_pos + (x_offset * SCALE) + sx;
                                let absolute_y = self.row_pos + (y_offset * SCALE) + sy;
                                self.draw_pixel(absolute_x, absolute_y);
                            }
                        }
                    }
                }
            }
        }
        self.column_pos += TOTAL;
        if self.column_pos >= self.info.width || c == '\n'
        {
            self.new_line()

        }
        self.draw_cursor();
    }

    fn draw_cursor(&mut self)
    {

        for x in self.column_pos..self.column_pos +2
        {
            for y in self.row_pos .. self.row_pos +TOTAL
            {
                self.draw_pixel(x, y);

            }
        }
    }
    fn clear_cursor(&mut self)
    {
        let color = self.rbga;
        self.rbga = Rgba::new(0x0);
        for x in self.column_pos..self.column_pos +TOTAL
        {
            for y in self.row_pos .. self.row_pos +TOTAL
            {
                self.draw_pixel(x, y);

            }
        }
        self.rbga = color;

    }
    pub(crate) fn toggle_cursor(&mut self)
    {
        self.cursor_visible = !self.cursor_visible;
        if self.cursor_visible {
            self.draw_cursor();
        }
        else {
            self.clear_cursor();
        }
    }
    pub(crate) fn delete_char(&mut self) {
        self.clear_cursor();

        if self.column_pos >= TOTAL {
            self.column_pos -= TOTAL;
        } else {
            if self.row_pos == 0 {

                self.draw_cursor();
                return;
            }
            self.row_pos -= TOTAL;
            self.column_pos = self.info.width - TOTAL;

        }

        let current_color = self.rbga;
        self.rbga = Rgba::new(0x00000000);

        for x in self.column_pos..(self.column_pos + TOTAL) {
            for y in self.row_pos..(self.row_pos + TOTAL) {
                self.draw_pixel(x, y);
            }
        }
        self.row_pos -= TOTAL;
        self.column_pos = self.info.width - TOTAL;

        self.rbga = current_color;

        self.draw_cursor();
    }


fn write_string(&mut self, s: &str)
    {
        for c in s.chars()
        {
            self.write_char(c);
        }
    }
    pub fn clear_screen(&mut self)
    {
        for i in 0..self.frame_buffer.len()
        {
            self.frame_buffer[i] = 0;
        }
    }

}

lazy_static! {
    pub static ref WRITER: Mutex<Option<Writer>> = Mutex::new(None);
}

pub fn init(buffer : &'static mut [u8], info : FrameBufferInfo)
{
    let writer = Writer::new(buffer, info, 0xF2F2F2FF);
    let mut guard = WRITER.lock();
    *guard = Some(writer);
}
impl fmt::Write for Writer
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);

        Ok(())
    }
}


#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::framebuffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts( ||  {
        let mut locked = WRITER.lock();
        if let Some(writer) = locked.as_mut() {
            writer.write_fmt(args).unwrap();
        }
    });

}