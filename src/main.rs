use generational_arena::Index;
use ynwm::*;

struct ViewData {
    view: Index,
    rect: Rect,
    mapped: bool,
}
enum CursorMode {
    Move,
    Resize,
    Passthrough,
}
fn view_at<'a>(server: &'a Server, views: &[ViewData], x: i32, y: i32) -> Option<(Index, SurfaceHit<'a>)> {
    views.iter().rev().find_map(|v| {
        let view = server.get_view(v.view);
        let rel_x = (x - v.rect.x) as f64;
        let rel_y = (y - v.rect.y) as f64;
        view.surface_at(rel_x, rel_y).map(|sh| {
            (v.view, sh)
        })
    })
}
fn main() {
    let mut ctx = ynwm::Server::new().expect("failed to create context");
    let mut views = Vec::new();
    let mut cursor_mode = CursorMode::Passthrough;
    loop {
        for e in ctx.as_mut().poll_events() {
            match e {
                e @ Event::CursorMotion { .. } | e @ Event::CursorMotionAbsolute { .. } => {
                    let time_ms = match e {
                        Event::CursorMotion {
                            time_ms,
                            delta_x,
                            delta_y,
                        } => {
                            ctx.as_mut().cursor_move(delta_x, delta_y);
                            time_ms
                        }
                        Event::CursorMotionAbsolute { time_ms, x, y } => {
                            ctx.as_mut().cursor_move_absolute(x, y);
                            time_ms
                        }
                        _ => unreachable!(),
                    };
                    match cursor_mode {
                        CursorMode::Passthrough => {
                            let (x, y) = ctx.as_ref().get_cursor();
                            if let Some((view, hit)) = view_at(&ctx, &views, x as i32, y as i32) {
                                ctx.pointer_notify_enter(&hit.surface, hit.hx, hit.hy);
                                ctx.pointer_notify_motion(time_ms, hit.hx, hit.hy);
                                ctx.as_mut().set_cursor_image("right_ptr");
                            } else {
                                ctx.pointer_clear_focus();
                                ctx.as_mut().set_cursor_image("left_ptr");
                            }
                        }
                        CursorMode::Move => {
                        }
                        CursorMode::Resize => {
                        }
                    }
                },
                Event::CursorFrame => {
                    ctx.pointer_notify_frame();
                },
                Event::XdgSurfaceNew { view } => {
                    let v = ctx.get_view(view);
                    let rect = v.get_rect();
                    views.push(ViewData {
                        view,
                        rect,
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
                    let output = ctx.as_mut().get_output_mut(output);
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
