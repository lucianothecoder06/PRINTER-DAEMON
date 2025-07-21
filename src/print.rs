use rocket::serde::json::Json;
use rocket::{get, post};
use rusb::{Context, UsbContext};
use serde::Deserialize;
use std::error::Error as StdError;
use std::time::Duration;
use qrcode::QrCode;
use image::{Luma, DynamicImage};

/// Line: one printable line of text with formatting & optional QR
#[derive(Deserialize, Clone)]
pub struct Line {
    pub text: String,
    pub center: bool,
    pub bold: bool,
    pub double_size: bool,
    pub qr: Option<String>,
}

/// PrintInfo: full print job
#[derive(Deserialize)]
pub struct PrintInfo {
    pub name: String,
    pub pid: u16,
    pub vid: u16,
    pub lines: Vec<Line>,
}

/// Generate a QR code as a bitmap image
fn generate_qr(data: &str) -> DynamicImage {
    let code = QrCode::new(data).unwrap();
    let img = code.render::<Luma<u8>>().build();
    DynamicImage::ImageLuma8(img)
}

/// Convert QR image to ESC/POS bytes
fn qr_to_escpos(image: DynamicImage) -> Vec<u8> {
    let mut bytes = Vec::new();

    let qr_resized = image.resize(200, 200, image::imageops::FilterType::Nearest);
    let gray = qr_resized.to_luma8();

    bytes.extend_from_slice(b"\x1D\x76\x30\x00"); // GS v 0

    let width_bytes = ((gray.width() + 7) / 8) as u16;
    let height = gray.height() as u16;

    bytes.push((width_bytes & 0xFF) as u8);
    bytes.push((width_bytes >> 8) as u8);
    bytes.push((height & 0xFF) as u8);
    bytes.push((height >> 8) as u8);

    for y in 0..gray.height() {
        for x_byte in 0..width_bytes {
            let mut b = 0u8;
            for bit in 0..8 {
                let x = x_byte * 8 + bit;
                if x < gray.width() as u16 {
                    let pixel = gray.get_pixel(x as u32, y);
                    if pixel[0] < 128 {
                        b |= 1 << (7 - bit);
                    }
                }
            }
            bytes.push(b);
        }
    }

    bytes
}

/// Compose ESC/POS bytes from lines
fn compose_print_data(lines: Vec<Line>) -> Vec<u8> {
    let mut data = Vec::new();

    data.extend_from_slice(b"\x1B\x40"); // Initialize printer

    for line in lines {
        // Alignment
        if line.center {
            data.extend_from_slice(b"\x1B\x61\x01");
        } else {
            data.extend_from_slice(b"\x1B\x61\x00");
        }

        // Bold
        if line.bold {
            data.extend_from_slice(b"\x1B\x45\x01");
        } else {
            data.extend_from_slice(b"\x1B\x45\x00");
        }

        // Double size
        if line.double_size {
            data.extend_from_slice(b"\x1D\x21\x11");
        } else {
            data.extend_from_slice(b"\x1D\x21\x00");
        }

        // Text
        if !line.text.is_empty() {
            data.extend_from_slice(line.text.as_bytes());
            data.push(b'\n');
        }

        // QR
        if let Some(qr_content) = &line.qr {
            let qr_img = generate_qr(qr_content);
            let qr_bytes = qr_to_escpos(qr_img);
            data.extend_from_slice(&qr_bytes);
            data.push(b'\n');
        }
    }

    // Reset
    data.extend_from_slice(b"\x1B\x45\x00");
    data.extend_from_slice(b"\x1D\x21\x00");
    data.extend_from_slice(b"\x1B\x61\x00");

    // Cut
    data.extend_from_slice(b"\x1B\x64\x03");
    data.extend_from_slice(b"\x1D\x56\x00");

    data
}

/// Send data to the printer
fn print_to_thermal_printer(vid: u16, pid: u16, lines: Vec<Line>) -> Result<(), Box<dyn StdError>> {
    let context = Context::new()?;

    for device in context.devices()?.iter() {
        let desc = device.device_descriptor()?;
        if desc.vendor_id() == vid && desc.product_id() == pid {
            println!("Found device: {:04x}:{:04x}", vid, pid);

            let handle = device.open()?;

            let interface_number = 0;
            if handle.kernel_driver_active(interface_number)? {
                handle.detach_kernel_driver(interface_number)?;
                println!("Detached kernel driver from interface {}", interface_number);
            }

            handle.claim_interface(interface_number)?;
            println!("Claimed interface {}", interface_number);

            let config_desc = device.active_config_descriptor()?;
            let mut bulk_out_endpoint = None;

            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    for endpoint_desc in interface_desc.endpoint_descriptors() {
                        if endpoint_desc.direction() == rusb::Direction::Out
                            && endpoint_desc.transfer_type() == rusb::TransferType::Bulk
                        {
                            bulk_out_endpoint = Some(endpoint_desc.address());
                        }
                    }
                }
            }

            if let Some(endpoint) = bulk_out_endpoint {
                println!("Using bulk out endpoint: 0x{:02x}", endpoint);

                let print_data = compose_print_data(lines);

                let timeout = Duration::from_secs(5);
                let bytes_written = handle.write_bulk(endpoint, &print_data, timeout)?;
                println!("Sent {} bytes to printer", bytes_written);

                handle.release_interface(interface_number)?;
                println!("Released interface {}", interface_number);

                return Ok(());
            } else {
                return Err("No suitable bulk out endpoint found".into());
            }
        }
    }

    Err("Thermal printer not found".into())
}

#[get("/print")]
pub fn print_receipt() -> String {
    match print_to_thermal_printer(0x0FE6, 0x811E, Vec::new()) {
        Ok(_) => println!("Printed successfully."),
        Err(e) => eprintln!("Printing failed: {}", e),
    }
    format!("User: oscar")
}

#[post("/print", format = "json", data = "<print_info>")]
pub fn print_receipt_info(print_info: Json<PrintInfo>) -> String {
    println!(
        "Received VID: {}, PID: {}, Name: {}",
        print_info.vid, print_info.pid, print_info.name
    );

    match print_to_thermal_printer(print_info.vid, print_info.pid, print_info.lines.clone()) {
        Ok(_) => println!("Printed successfully."),
        Err(e) => eprintln!("Printing failed: {}", e),
    }
    format!("User: {}", print_info.name)
}
