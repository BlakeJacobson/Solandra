// use winit::{
//     application::ApplicationHandler,
//     event::{KeyEvent, WindowEvent},
//     event_loop::{self, EventLoop},
//     window::Window,
// };

// pub struct App {
//     pub window: Option<Arc<Window>>,
// }

// impl App {
//     pub async fn new() -> Self {
//         Self { window: None }
//     }
//     pub fn run(mut self) -> anyhow::Result<()> {
//         let event_loop = EventLoop::new()?;
//         event_loop.run_app(&mut self)?;
//         Ok(())
//     }
// }

// impl ApplicationHandler for Engine {

// }
