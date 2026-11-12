mod game;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    game::initgame()?;
    Ok(())
}
