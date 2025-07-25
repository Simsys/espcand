#![no_main]
#![no_std]
#[rtic::app(device = esp32c3, dispatchers=[FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use esp_backtrace as _;
    use esp_hal::gpio::{Event, Input, InputConfig, Pull};
    use defmt::println;
    use defmt_rtt as _;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        button: Input<'static>,
    }

    // do nothing in init
    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        println!("init");

        let peripherals = esp_hal::init(esp_hal::Config::default());
        let mut button = Input::new(
            peripherals.GPIO9,
            InputConfig::default().with_pull(Pull::Up),
        );
        button.listen(Event::FallingEdge);
        foo::spawn().unwrap();
        (Shared {}, Local { button })
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        println!("idle");
        loop {}
    }

    #[task(priority = 5)]
    async fn foo(_: foo::Context) {
        bar::spawn().unwrap(); //enqueue low prio task
        println!("Inside high prio task, press button now!");
        let mut x = 0;
        while x < 50_000_000 {
            x += 1; //burn cycles
            esp_hal::riscv::asm::nop();
        }
        println!("Leaving high prio task. {}", x);
    }

    #[task(priority = 2)]
    async fn bar(_: bar::Context) {
        println!("Inside low prio task, press button now!");
        let mut x = 0;
        while x < 50_000_000 {
            x += 1; //burn cycles
            esp_hal::riscv::asm::nop();
        }
        println!("Leaving low prio task. {}", x);
    }

    #[task(binds=GPIO, local=[button], priority = 3)]
    fn gpio_handler(cx: gpio_handler::Context) {
        cx.local.button.clear_interrupt();
        println!("button");
    }
}
