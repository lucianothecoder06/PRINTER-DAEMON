#[macro_use]
extern crate rocket;

mod print;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .configure(rocket::Config {
            address: std::net::Ipv4Addr::LOCALHOST.into(),
            port: 5978,
            ..Default::default()
        })
        .mount("/", routes![index, print::print_receipt, print::print_receipt_info])
}
