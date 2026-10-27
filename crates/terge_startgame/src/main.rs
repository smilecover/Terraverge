use terge_client::{EventLoopError, RunGame};

fn main() -> Result<(), EventLoopError> {
    RunGame::new().run()
}
