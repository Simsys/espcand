use embassy_net::{Runner, StackResources, Stack};
use embassy_sync::{
        blocking_mutex::raw::CriticalSectionRawMutex, 
        watch::{Watch, Sender, Receiver},
};

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    Async,
    clock::CpuClock, 
    rng::Rng, 
    timer::timg::TimerGroup,
    twai::{self, filter::SingleStandardFilter, TwaiMode, Twai},
};
use esp_radio::{
    EspRadioController,
    wifi::{WifiController, WifiDevice},
};

use corelib::*;

const CAN_BAUDRATE: &str = env!("CAN_BAUDRATE");

pub fn init() ->
(
    Runner<'static, WifiDevice<'static>>,
    Stack<'static>,
    WifiController<'static>, 
    Twai<'static, Async>,
    &'static ComChannel,
    &'static ComChannel,
    Receiver<'static, CriticalSectionRawMutex, bool, 1>,
    Sender<'static, CriticalSectionRawMutex, bool, 1>,
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

    let tx_pin = peripherals.GPIO3;
    let rx_pin = peripherals.GPIO2;

    let baud_rate = match CAN_BAUDRATE {
        "B125K" => twai::BaudRate::B125K,
        "B250K" => twai::BaudRate::B250K,
        "B500K" => twai::BaudRate::B500K,
        _ => twai::BaudRate::B1000K,
    };
    
    let mut twai_config = twai::TwaiConfiguration::new(
        peripherals.TWAI0,
        rx_pin,
        tx_pin,
        baud_rate,
        TwaiMode::Normal,
    ).into_async();
    twai_config.set_filter(
        const { SingleStandardFilter::new(b"xxxxxxxxxxx", b"x", [b"xxxxxxxx", b"xxxxxxxx"]) },
    );
    let twai: Twai<'_, Async> = twai_config.start();

    let can_rx_channel = &*mk_static!(ComChannel, ComChannel::new());
    let can_tx_channel = &*mk_static!(ComChannel, ComChannel::new());

    static SIGNAL_CONN: Watch<CriticalSectionRawMutex, bool, 1> = Watch::new();
    let signal_conn_rx: Receiver<'static, CriticalSectionRawMutex, bool, 1> = SIGNAL_CONN.receiver().unwrap();
    let signal_conn_tx: Sender<'static, CriticalSectionRawMutex, bool, 1> = SIGNAL_CONN.sender();
    
    (
        runner,
        stack,
        controller,
        twai,
        can_rx_channel,
        can_tx_channel,
        signal_conn_rx,
        signal_conn_tx,
    )
}


