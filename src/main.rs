#[macro_use]
extern crate rocket;
use auto_launch::*;
mod print;
use std::env;
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    let exe_path = env::current_exe().expect("Failed to get current executable path");

    let auto = AutoLaunchBuilder::new()
        .set_app_name("printer-service")
        .set_app_path(exe_path.to_str().unwrap()) // use your current binary's path
        .set_use_launch_agent(true)
        .set_args(&["--minimized"])
        .build()
        .unwrap();

    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    rocket::build()
        .configure(rocket::Config {
            address: std::net::Ipv4Addr::LOCALHOST.into(),
            port: 5978,
            ..Default::default()
        })
        .mount(
            "/",
            routes![index, print::print_receipt, print::print_receipt_info],
        )
}
