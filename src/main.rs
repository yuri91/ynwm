fn main() {
    let mut ctx = ynwm::Server::new().expect("failed to create context");
    ctx.as_mut().main_loop();
}
