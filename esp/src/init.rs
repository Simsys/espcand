use embassy_net::{Runner, StackResources, Stack};

use embassy_sync::pipe::Pipe;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_radio::{
    EspRadioController,
    wifi::{WifiController, WifiDevice},
};

use corelib::WifiPipe;

pub fn init() ->
(
    Runner<'static, WifiDevice<'static>>,
    Stack<'static>,
    WifiController<'static>, 
    &'static WifiPipe,
    &'static WifiPipe,
) {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    // init esp radio
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_radio_preempt_baremetal::init(timg0.timer0);

    // create wifi interface
    let esp_wifi_ctrl = &*mk_static!(EspRadioController<'static>, esp_radio::init().unwrap());
    let (controller, interfaces) = esp_radio::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();
    let wifi_interface = interfaces.sta;

    // get timer for embassy
    use esp_hal::timer::systimer::SystemTimer;
    let systimer = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(systimer.alarm0);

    // Init network stack and runner
    let config = embassy_net::Config::dhcpv4(Default::default());
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    let wifi_rx_data = &*mk_static!(WifiPipe, Pipe::new());
    let wifi_tx_data = &*mk_static!(WifiPipe, Pipe::new());

    (
        runner,
        stack,
        controller,
        wifi_rx_data,
        wifi_tx_data,
    )
}


