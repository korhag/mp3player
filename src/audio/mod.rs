// ------------------------------------------------------------
// Module: player
// Purpose: Owns the audio device on a background thread so the
//          window never blocks on decoding or the sound driver.
// ------------------------------------------------------------

mod player;
mod stream;

pub use player::{AudioHandle, Phase};
