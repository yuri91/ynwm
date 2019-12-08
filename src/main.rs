use generational_arena::Index;
use std::pin::Pin;
use ynwm::*;

struct ViewData {
    view: Index,
    rect: Rect,
    mapped: bool,
}
fn main() {
    let mut ctx = ynwm::Server::new().expect("failed to create context");
    let mut views = Vec::new();
    loop {
        for e in ctx.as_mut().poll_events() {
            match e {
                Event::XdgSurfaceNew { view } => {
                    views.push(ViewData {
                        view,
                        rect: Rect {
                            x: 0,
                            y: 0,
                            w: 0,
                            h: 0,
                        },
                        mapped: false,
                    });
                }
                Event::XdgSurfaceDestroy { view } => {
                    let idx = views
                        .iter()
                        .position(|i| i.view == view)
                        .expect("view not found");
                    views.remove(idx);
                }
                Event::XdgSurfaceMap { view } => {
                    let idx = views
                        .iter()
                        .position(|i| i.view == view)
                        .expect("view not found");
                    views[idx].mapped = true;
                }
                Event::XdgSurfaceUnmap { view } => {
                    let idx = views
                        .iter()
                        .position(|i| i.view == view)
                        .expect("view not found");
                    views[idx].mapped = false;
                }
                Event::OutputFrame { output, when } => {
                    let output = ctx.as_mut().get_output(output);
                    let to_render = views.iter().filter_map(|v| {
                        if !v.mapped {
                            None
                        } else {
                            Some((v.view, v.rect))
                        }
                    });
                    output.render_views(to_render);
                }
                _ => {
                    println!("{:?}", e);
                }
            }
        }
    }
}
