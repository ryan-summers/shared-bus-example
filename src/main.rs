#![no_std]
#![no_main]

use panic_halt as _;

use rtic::cyccnt::Duration;
use stm32h7xx_hal::prelude::*;

mod si1145;

use si1145::Si1145;
use si7021::Si7021;

type I2c = stm32h7xx_hal::i2c::I2c<stm32h7xx_hal::stm32::I2C2>;
type I2cProxy = shared_bus::I2cProxy<'static, shared_bus::AtomicCheckMutex<I2c>>;

pub struct I2cDevices {
    pub si7021: Si7021<I2cProxy>,
    pub si1145: Si1145<I2cProxy>,
}

#[rtic::app(device = stm32h7xx_hal::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        devices: I2cDevices,
    }

    #[init(schedule=[fast_task, slow_task])]
    fn init(mut c: init::Context) -> init::LateResources {
        c.core.DWT.enable_cycle_counter();

        let rcc = c.device.RCC.constrain();
        let pwr = c.device.PWR.constrain();
        let vos = pwr.freeze();

        let ccdr = rcc
            .sysclk(400.mhz())
            .hclk(200.mhz())
            .per_ck(100.mhz())
            .freeze(vos, &c.device.SYSCFG);

        // Create the shared-bus I2C manager.
        let bus_manager: &'static _ = {
            let gpiof = c.device.GPIOF.split(ccdr.peripheral.GPIOF);
            let i2c_sda = gpiof.pf0.into_open_drain_output().into_alternate_af4();
            let i2c_scl = gpiof.pf1.into_open_drain_output().into_alternate_af4();

            let i2c = c.device.I2C2.i2c(
                (i2c_scl, i2c_sda),
                100.khz(),
                ccdr.peripheral.I2C2,
                &ccdr.clocks,
            );

            shared_bus::new_atomic_check!(I2c = i2c).unwrap()
        };

        // Create the devices.
        let devices = {
            let si7021 = Si7021::new(bus_manager.acquire_i2c());
            let si1145 = Si1145::new(bus_manager.acquire_i2c());

            I2cDevices { si7021, si1145 }
        };

        // Kick start the tasks.
        c.schedule.fast_task(c.start).unwrap();
        c.schedule.slow_task(c.start).unwrap();

        init::LateResources { devices }
    }

    #[task(priority=3, schedule=[fast_task], resources=[devices])]
    fn fast_task(c: fast_task::Context) {
        let _part_id = c.resources.devices.si1145.read_part_id().unwrap();

        // Run this task at 50Hz.
        c.schedule
            .fast_task(c.scheduled + Duration::from_cycles(400_000_000 / 50))
            .unwrap();
    }

    #[task(priority=2, schedule=[slow_task], resources=[devices])]
    fn slow_task(mut c: slow_task::Context) {
        let _temp_c = c
            .resources
            .devices
            .lock(|devices| devices.si7021.temperature_celsius().unwrap());

        // Run this task at 5Hz.
        c.schedule
            .slow_task(c.scheduled + Duration::from_cycles(400_000_000 / 5))
            .unwrap();
    }

    #[idle(resources=[devices])]
    fn idle(mut c: idle::Context) -> ! {
        loop {
            c.resources.devices.lock(|devices| {
                let _temp_c = devices.si7021.temperature_celsius().unwrap();
                let _part_id = devices.si1145.read_part_id().unwrap();
            });
        }
    }

    extern "C" {
        fn EXTI0();
        fn EXTI1();
    }
};
