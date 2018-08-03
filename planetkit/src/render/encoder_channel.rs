use gfx;
use std::sync::mpsc;

// Bi-directional channel to send Encoder between game thread(s)
// (as managed by Specs), and the thread owning the graphics device.
pub struct EncoderChannel<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    pub receiver: mpsc::Receiver<gfx::Encoder<R, C>>,
    pub sender: mpsc::Sender<gfx::Encoder<R, C>>,
}
