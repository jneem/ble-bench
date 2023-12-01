#![no_std]
#![no_main]

use core::cell::{Cell, RefCell};

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    attribute_server::{AttributeServer, WorkResult},
    gatt, Ble, HciConnector,
};
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{ble::controller::BleConnector, initialize, EspWifiInitFor};
use hal::{clock::ClockControl, peripherals::*, prelude::*, systimer::SystemTimer, Rng};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Ble,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let mut bluetooth = peripherals.BT;

    loop {
        let connector = BleConnector::new(&init, &mut bluetooth);
        let hci = HciConnector::new(connector, esp_wifi::current_millis);
        let mut ble = Ble::new(&hci);

        println!("{:?}", ble.init());
        println!("{:?}", ble.cmd_set_le_advertising_parameters());
        println!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName("ESP32-C3"),
                ])
                .unwrap()
            )
        );
        println!("{:?}", ble.cmd_set_le_advertise_enable(true));

        println!("received started advertising");

        // The client sends packets in bursts of 50, after which we report the bandwidth.
        let packet_count = Cell::new(0);
        let data_size = Cell::new(0);
        let start_time = Cell::new(0);
        let last_time = Cell::new(0);

        let mut report = |_offset: usize, data: &mut [u8]| {
            println!(
                "read {bytes} bytes in {ms} ms",
                bytes = data_size.get(),
                ms = (last_time.get() - start_time.get()) * 1000 / SystemTimer::TICKS_PER_SECOND
            );
            packet_count.set(0);
            data.fill(0);
            data.len()
        };

        let mut rf = |_offset: usize, data: &mut [u8]| {
            data.fill(42);
            data.len()
        };
        let mut wf = |offset: usize, data: &[u8]| {
            if offset == 0 {
                let pc = packet_count.get();
                packet_count.set(pc + 1);
                if pc == 0 {
                    start_time.set(SystemTimer::now());
                    data_size.set(0);
                }
            }
            data_size.set(data_size.get() + data.len());
            last_time.set(SystemTimer::now());
        };

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [
                characteristic {
                    uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
                    read: rf,
                    write: wf,
                },
                characteristic {
                    uuid: "957312e0-2354-11eb-9f10-fbc30a62cf38",
                    read: report,
                },
            ],
        },]);

        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes);

        loop {
            match srv.do_work() {
                Ok(res) => {
                    if let WorkResult::GotDisconnected = res {
                        break;
                    }
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        }
    }
}
