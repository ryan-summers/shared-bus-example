use embedded_hal::blocking::i2c::WriteRead;

pub struct Si1145<I2C>
where
    I2C: WriteRead,
{
    i2c: I2C,
}

impl<I2C> Si1145<I2C>
where
    I2C: WriteRead,
{
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    pub fn read_part_id(&mut self) -> Result<u8, I2C::Error> {
        let mut data = [0u8];
        self.i2c.write_read(0x60, &[0x00], &mut data)?;
        Ok(data[0])
    }
}
