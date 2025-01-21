#[derive(Debug)]
pub enum Event {
    SetDiffusionA(f32),
    SetDiffusionB(f32),
    SetFeed(f32),
    SetKill(f32),
    SetStepsPerFrame(u32),
    SetTimestep(f32),
    Reset,
    Start,
    Pause,
}
