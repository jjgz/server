use super::net;
use std::sync::mpsc::{Receiver, TryRecvError};

use glium;
use glowygraph;

#[derive(Debug, Clone, Default)]
struct World {
    frame: u32,
    total: Option<u32>,
    pieces: Vec<net::WorldPiece>,
}

fn color_line(p0: &net::EndPoint,
              p1: &net::EndPoint,
              color: [f32; 4])
              -> [glowygraph::render2::Node; 2] {
    let p0 = &p0.point;
    let p1 = &p1.point;
    [glowygraph::render2::Node {
         position: [p0.x, p0.y],
         inner_color: color,
         falloff: 0.1,
         falloff_color: color,
         falloff_radius: p0.v.sqrt() * 2.0,
         inner_radius: 0.0,
     },
     glowygraph::render2::Node {
         position: [p1.x, p1.y],
         inner_color: color,
         falloff: 0.1,
         falloff_color: color,
         falloff_radius: p1.v.sqrt() * 2.0,
         inner_radius: 0.0,
     }]
}

fn color_point(p: &net::Point, color: [f32; 4]) -> [glowygraph::render2::Node; 1] {
    [glowygraph::render2::Node {
         position: [p.x, p.y],
         inner_color: color,
         falloff: 0.1,
         falloff_color: color,
         falloff_radius: p.v.sqrt() * 2.0,
         inner_radius: 0.0,
     }]
}

pub fn render(recv: Receiver<net::World>) {
    use glium::DisplayBuild;
    use glowygraph::render2::Renderer;

    let display = glium::glutin::WindowBuilder::new().with_vsync().build_glium().unwrap();
    let glowy = Renderer::new(&display);

    let mut last_constructed_world = World {
        frame: 0,
        total: Some(1),
        pieces: vec![net::WorldPiece::ArenaBorder {
                         p0: net::EndPoint {
                             point: net::Point {
                                 x: -0.5,
                                 y: -0.5,
                                 v: 0.001,
                             },
                             open: false,
                         },
                         p1: net::EndPoint {
                             point: net::Point {
                                 x: -0.5,
                                 y: -0.5,
                                 v: 0.001,
                             },
                             open: false,
                         },
                     }],
    };

    let mut newest_world = last_constructed_world.clone();

    loop {
        use glium::Surface;
        // Update world model.
        loop {
            match recv.try_recv() {
                Ok(w) => {
                    if w.frame > newest_world.frame {
                        // This is a newer frame we are getting, so forget the old one and make a new one.
                        newest_world = World::default();
                    } else if w.frame < newest_world.frame {
                        // This doesn't belong.
                        continue;
                    }

                    match w.piece {
                        net::WorldPiece::Total(n) => newest_world.total = Some(n),
                        wp => newest_world.pieces.push(wp),
                    }

                    if let Some(n) = newest_world.total {
                        if newest_world.pieces.len() == n as usize {
                            last_constructed_world = newest_world.clone();
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    panic!("Server world channel disconnected.");
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
            }
        }

        // Get dimensions
        let dims = display.get_framebuffer_dimensions();
        // Multiply this by width coordinates to get normalized screen coordinates.
        let hscale = dims.1 as f32 / dims.0 as f32;
        // Use the projection matrix to scale the screen so that
        // y goes from [-1, 1) and x goes from [-hscale, hscale).
        let projection = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        // Scale dimensions down by 5 times.
        let modelview = [[1.0 / 5.0 / hscale, 0.0, 0.0], [0.0, 1.0 / 5.0, 0.0], [0.0, 0.0, 1.0]];

        // Begin rendering.
        let mut target = display.draw();

        target.clear_color(0.0, 0.0, 0.0, 1.0);

        for e in &last_constructed_world.pieces {
            match *e {
                net::WorldPiece::ArenaBorder { ref p0, ref p1 } => {
                    glowy.render_edges_round(&mut target,
                                             modelview,
                                             projection,
                                             &color_line(p0, p1, [1.0, 0.0, 0.0, 1.0]));
                }
                net::WorldPiece::VisibilityBorder { ref p0, ref p1 } => {
                    glowy.render_edges_round(&mut target,
                                             modelview,
                                             projection,
                                             &color_line(p0, p1, [0.0, 0.0, 1.0, 1.0]));
                }
                net::WorldPiece::ObjectBorder { ref p0, ref p1 } => {
                    glowy.render_edges_round(&mut target,
                                             modelview,
                                             projection,
                                             &color_line(p0, p1, [0.0, 1.0, 0.0, 1.0]));
                }
                net::WorldPiece::Target(ref p) => {
                    glowy.render_nodes(&mut target,
                                       modelview,
                                       projection,
                                       &color_point(p, [1.0, 0.0, 0.0, 1.0]));
                }
                net::WorldPiece::RoverA(ref p) => {
                    glowy.render_nodes(&mut target,
                                       modelview,
                                       projection,
                                       &color_point(&p.point, [0.0, 1.0, 0.0, 1.0]));
                }
                net::WorldPiece::RoverB(ref p) => {
                    glowy.render_nodes(&mut target,
                                       modelview,
                                       projection,
                                       &color_point(&p.point, [0.0, 0.0, 1.0, 1.0]));
                }
                _ => {}
            }
        }

        // Vsync, end rendering, and flip buffer.
        target.finish().unwrap();

        // Don't do anything with events, but clear them out.
        for _ in display.poll_events() {
        }
    }
}
