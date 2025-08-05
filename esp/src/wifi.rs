
use embassy_futures::select::{select, Either};
use embassy_net::{tcp::TcpSocket, Runner, Stack};
use embassy_time::{Duration, Timer};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    watch::Sender,
};

use esp_alloc as _;
use esp_backtrace as _;
use esp_println::println;
use esp_radio::wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState};

use embedded_io_async::Write;
use log::{info, warn};

use corelib::{ComChannel, ComItem, DeSer, Error, RxBuffer, Serialize}; 


const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[embassy_executor::task]
pub async fn comm(
    stack: Stack<'static>,
    wifi_rx_channel: &'static ComChannel,
    wifi_tx_channel: &'static ComChannel,
    set_connection: Sender<'static, CriticalSectionRawMutex, bool, 1>
) {
    let rx_buffer = mk_static!([u8; 4096], [0; 4096]);
    let tx_buffer = mk_static!([u8; 4096], [0; 4096]);
    let mut rxbuf = RxBuffer::<2048>::new();
    //let mut txbuf = [0_u8; 4096];
    set_connection.send(false);

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
        set_connection.send(true);
        info!("Received connection from {:?}", socket.remote_endpoint());

        loop {
            if !socket.may_recv() {
                set_connection.send(false);
                socket.abort();
                warn!("Connection closed");
                break;
            }

            match socket_write_read(
                &mut socket, 
                wifi_rx_channel, 
                wifi_tx_channel,
                &mut rxbuf,
            ).await {
                Ok(()) => (),
                Err(error) => wifi_tx_channel.send(ComItem::Error(error)).await,
            }
        }
    }
}


async fn socket_write_read<'a>(
    socket: &'a mut TcpSocket<'static>, 
    wifi_rx_channel: &'static ComChannel,
    wifi_tx_channel: &'static ComChannel,
    rxbuf: &mut RxBuffer<2048>,
) -> Result <(), Error> {
    let socket_write = async {
        wifi_tx_channel.receive().await
    };
    let socket_read = async {
        match socket.read(rxbuf.as_mut_slice()).await {
            Ok(n) => n,
            Err(_) => 0,
        }
    };

    // Wait for both and handle first event 
    match select(socket_write, socket_read).await {
        Either::First(com_item) => {
            let ser = com_item.serialize();
            match socket.write_all(ser.as_slice()).await {
                Ok(()) => (),
                Err(_) => println!("Socket write error"),
            };
        }
        Either::Second(n) => {
            rxbuf.set_head(n);            
            loop {
                let mut de_ser = DeSer::<50>::new();
                match rxbuf.read(&mut de_ser) {
                    Ok(()) => (),
                    Err(e) => {
                        match e {
                            Error::BufIsEmpty => return Ok(()),
                            _ => return Err(e),
                        }
                    }
                }
                let item = ComItem::deserialize(&mut de_ser)?;
                wifi_rx_channel.send(item).await;
            }
        }
    };
    Ok(())
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