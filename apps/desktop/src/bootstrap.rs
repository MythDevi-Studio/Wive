use std::path::PathBuf;

use winit::event_loop::EventLoop;

use crate::app::WiveApp;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).map(PathBuf::from);
    let mut app = WiveApp::new(path);
    EventLoop::new()?.run_app(&mut app)?;
    Ok(())
}
