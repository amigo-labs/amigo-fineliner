//! Stateless image effects for Fineliner (spec §11).
//!
//! This crate is independent of `fineliner-core` (ADR-002): every effect is a
//! pure function from an [`EffectImage`] to a new [`EffectImage`], with no
//! document, no I/O, and no shared state. That makes previews (apply to a
//! downscaled copy), parallelism, and testing trivial.
//!
//! # Conventions
//!
//! - **RGBA8 in, RGBA8 out.** Callers pass a layer's straight-alpha RGBA8 bytes
//!   and get back a same-sized buffer.
//! - **f32 internally.** Effects convert to f32 in `[0.0, 1.0]` on entry and
//!   back on exit (spec §4.1).
//! - **Premultiplied alpha for spatial effects.** Convolutions and resampling
//!   run in premultiplied alpha so the arbitrary colour of fully transparent
//!   pixels never bleeds into opaque neighbours. See
//!   [`EffectImage::to_premultiplied_f32`].
//! - **Gamma (sRGB) space.** Effects operate on sRGB samples directly; they do
//!   not linearise. This matches paint.net's behaviour and keeps the effect
//!   pipeline independent of the compositor's linear-light blending (ADR-015).
//!
//! # Adding an effect
//!
//! Define a parameter struct and implement [`Effect`]: `apply` at full
//! resolution, and `scaled` so [`Effect::preview`] can run on a downscaled copy
//! with matching parameters.

mod effect;
mod error;
mod image;
mod kernel;

pub mod adjust;
pub mod blur;
pub mod distort;
pub mod noise;
pub mod sharpen;

pub use effect::Effect;
pub use error::EffectError;
pub use image::EffectImage;
