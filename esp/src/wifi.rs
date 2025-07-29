
use embassy_futures::select::{select, Either};
use embassy_net::{tcp::TcpSocket, Runner, Stack};
use embassy_time::{Duration, Timer};
use embassy_sync::{pipe::Pipe, blocking_mutex::raw::NoopRawMutex};

use embedded_io_async::Write;
use esp_alloc as _;
use esp_backtrace as _;
use esp_println::println;
use esp_radio::wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState};

use log::{info, warn};

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[embassy_executor::task]
pub async fn communication(
    stack: Stack<'static>,
    wifi_rx_data: &'static Pipe<NoopRawMutex, 4096>,
    wifi_tx_data: &'static Pipe<NoopRawMutex, 4096>,
) {
    let rx_buffer = mk_static!([u8; 4096], [0; 4096]);
    let tx_buffer = mk_static!([u8; 4096], [0; 4096]);
    let mut rxbuf = [0_u8; 1024];
    let mut txbuf = [0_u8; 1024];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    loop {
        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }
        info!("Received connection from {:?}", socket.remote_endpoint());

        loop {
            if !socket.may_recv() {
                socket.abort();
                warn!("Connection closed");
                break;
            }

            // define socket read and write future
            let socket_write = async {
                wifi_tx_data.read(&mut txbuf).await
            };
            let socket_read = async {
                match socket.read(&mut rxbuf).await {
                    Ok(n) => n,
                    Err(_) => 0,
                }
            };

            // Wait for both and handle first event 
            match select(socket_write, socket_read).await {
                Either::First(n) => {
                    let _ = socket.write_all(&txbuf[..n]).await;
                }
                Either::Second(n) => {
                    let _ = wifi_rx_data.write_all(&rxbuf[..n]).await;
                }
            };
        }
    }
}



#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_radio::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(_e) => {
                println!("Failed to connect to wifi");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}