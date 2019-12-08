fn main() {
    let mut ctx = ynwm::Server::new().expect("failed to create context");
    loop {
        for e in ctx.as_mut().poll_events() {
            println!("{:?}",e);
        }
    }
}
