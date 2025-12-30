#[macro_use]
extern crate rocket;

use auto_launch::*;
use std::env;

mod print;
mod cors;

use cors::Cors;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

// Catch-all OPTIONS handler for CORS preflight
#[options("/<_..>")]
fn options() {}

#[launch]
fn rocket() -> _ {
    let exe_path = env::current_exe().expect("Failed to get current executable path");

    let auto = AutoLaunchBuilder::new()
        .set_app_name("printer-service")
        .set_app_path(exe_path.to_str().unwrap())
        .set_use_launch_agent(true)
        .set_args(&["--minimized"])
        .build()
        .unwrap();

    let _ = auto.enable();
    let _ = auto.is_enabled();

    rocket::build()
        .configure(rocket::Config {
            address: std::net::Ipv4Addr::LOCALHOST.into(),
            port: 5978,
            ..Default::default()
        })
        .attach(Cors)
        .mount(
            "/",
            routes![
                index,
                options,
                print::print_receipt,
                print::print_receipt_info,
                print::print_receipt_options
            ],
        )
}
