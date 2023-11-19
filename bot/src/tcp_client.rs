use core::marker::PhantomData;
use core::str::FromStr;

use cyw43::NetDriver;
use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config as ConfigNet, Ipv4Address, Ipv4Cidr, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::USB;
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0};
use embassy_rp::pio::{InterruptHandler as InterruptHandlerPio, Pio};
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use heapless::Vec;
use rp2040_panic_usb_boot as _;
use static_cell::make_static;

const WIFI_SSID: &'static str = include_str!("./WIFI_SSID.txt");
const WIFI_SECRET: &'static str = include_str!("./WIFI_SECRET.txt");

#[embassy_executor::task]
pub async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, PIN_23>,
        PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

const SOCKET_BUFFER_SIZE: usize = 128;

#[non_exhaustive]
pub struct Connection {
    stack: &'static Stack<NetDriver<'static>>,
}

impl Connection {
    pub async fn init(
        spawner: &Spawner,
        pwr: Output<'static, PIN_23>,
        spi: PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    ) -> Connection {
        let fw = include_bytes!("../deps/cyw43-firmware/43439A0.bin");
        let clm = include_bytes!("../deps/cyw43-firmware/43439A0_clm.bin");

        let state = make_static!(cyw43::State::new());
        let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
        spawner.spawn(wifi_task(runner)).unwrap();
        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
            .await;

        // Generate random seed
        let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

        // Init network stack
        let config = ConfigNet::ipv4_static(embassy_net::StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::from_str("10.1.1.4").unwrap(), 24),
            dns_servers: Vec::new(),
            gateway: Some(Ipv4Address::from_str("10.1.1.1").unwrap()),
        });
        let stack = &*make_static!(Stack::new(
            net_device,
            config,
            make_static!(StackResources::<2>::new()),
            seed
        ));
        spawner.spawn(net_task(stack)).unwrap();

        // Join wifi network
        log::info!(
            "Joining access point {} (link up: {})",
            WIFI_SSID,
            stack.is_link_up()
        );
        loop {
            match control.join_wpa2(WIFI_SSID, WIFI_SECRET).await {
                Ok(_) => break,
                Err(err) => {
                    log::info!("join failed with status={}", err.status);
                    Timer::after(Duration::from_millis(1000)).await;
                }
            }
        }
        Self { stack }
    }

    pub async fn connect(&mut self) {
        // Connect to TCP server
        loop {
            log::info!("connecting...");
            let mut rx_buffer = [0u8; SOCKET_BUFFER_SIZE];
            let mut tx_buffer = [0u8; SOCKET_BUFFER_SIZE];
            let mut socket = TcpSocket::new(self.stack, &mut rx_buffer, &mut tx_buffer);
            socket.set_timeout(Some(Duration::from_secs(5)));
            let address = Ipv4Address::from_str("10.128.4.179").unwrap();
            if let Err(err) = socket.connect((address, 9001)).await {
                log::warn!("connection error: {:?}", err);
                Timer::after(Duration::from_millis(1000)).await;
                continue;
            }

            let msg = b"Hello world!\n";
            loop {
                log::info!("tx: {}", core::str::from_utf8(msg).unwrap());
                if let Err(err) = socket.write_all(msg).await {
                    log::warn!("connection error: {:?}", err);
                    break;
                }
                Timer::after(Duration::from_millis(1000)).await;
            }
        }
    }
}
