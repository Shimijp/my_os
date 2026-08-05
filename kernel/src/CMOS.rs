use core::fmt::{Display, Formatter};
use lazy_static::lazy_static;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::port::Port;
use crate::mutex::Mutex;

pub fn bcd(value: u8) -> u8
{

    let tens = (value  & 0xF0_u8) >> 4;
    let singles = value & 0x0F_u8;

    tens * 10 + singles

}
pub struct CMOS
{
    write_port: Port<u8>,
    read_port: Port<u8>,

}
impl CMOS
{
    pub fn new() -> Self
    {
        CMOS {
            write_port: Port::new(0x70),
            read_port: Port::new(0x71),
        }
    }
    fn read_register(&mut self, reg: u8) -> u8
    {
        unsafe
            {
                without_interrupts( || {
                    Port::write(&mut self.write_port, reg);
                    Port::read(&mut self.read_port)
                })

            }
    }
    pub fn get_time(&mut self) -> Time {
        without_interrupts( || {
            while self.read_register(0xA) & 0x80 != 0 {} //the data we need is at the highest bit
            // the highest bit is the interrupt bit
            let seconds = bcd(self.read_register(0x00 | 0x80));
            let minutes = bcd(self.read_register(0x02 | 0x80));
            let hours = bcd(self.read_register(0x04 | 0x80));
            Time
            {
                hours,
                minutes,
                seconds,
            }
        })

    }
    pub fn get_time_and_date(&mut self, offset: u8) -> TimeAndDate{
        let mut time = self.get_time();
        let date = self.get_date();
        time.hours += offset;
        time.hours %= 24;
        TimeAndDate{
            time,
            date
        }
    }


    pub fn get_date(&mut self) -> Date
    {
        without_interrupts(|| {
            while self.read_register(0xA) & 0x80 != 0 {}
            let day = bcd(self.read_register(0x07 | 0x80));
            let month = bcd(self.read_register(0x08 | 0x80));
            let year = bcd(self.read_register(0x09 | 0x80));
            Date {
                year,
                month,
                day,
            }
        })

    }
}

pub struct Time
{
    hours: u8,
    minutes: u8,
    seconds: u8,
}
impl Display for Time
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hours, self.minutes, self.seconds)
    }
}

pub struct Date
{
    year: u8,
    month: u8,
    day: u8,
}
impl Display for Date
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02}/{:02}/{:02}", self.day, self.month, self.year)

    }
}

pub struct TimeAndDate
{
    date: Date,
    time: Time,
}
impl Display for TimeAndDate
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}\n{}", self.time, self.date)
    }
}

lazy_static!
{
    pub static  ref  CMO :  Mutex<CMOS>   = Mutex::new(CMOS::new());
}

