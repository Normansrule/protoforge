//! # forge-core
//!
//! The Protoforge engineering engine. Every lab in the web app, the CLI and the
//! REST server call into this crate. It is dependency-light on purpose (only
//! `serde` + `serde_json`) so it compiles to WebAssembly and runs entirely in a
//! browser on GitHub Pages.
//!
//! The crate is organised the way a product moves through the systems
//! engineering "V":
//!
//! | Stage | Modules |
//! |---|---|
//! | Concept, needs, requirements, architecture, verification | [`project`], [`systems`] |
//! | Electrical design & simulation | [`circuit`], [`power`] |
//! | Signal processing | [`dsp`] |
//! | Digital / chip design | [`chip`] |
//! | Motion, estimation & control | [`control`], [`robotics`], [`rl`] |
//! | Optical design | [`optics`] |
//! | Mechanical CAD & 3D printing | [`cad`], [`print`] |
//! | PCB layout & fab outputs | [`pcb`] |
//! | Injection molding & manufacturing | [`molding`], [`mfg`] |
//!
//! All modules expose plain Rust functions plus a `dispatch` hook used by the
//! JSON API in [`api`].

pub mod api;
pub mod math;
pub mod project;

pub mod cad;
pub mod chip;
pub mod circuit;
pub mod control;
pub mod dsp;
pub mod mfg;
pub mod molding;
pub mod optics;
pub mod pcb;
pub mod power;
pub mod print;
pub mod rl;
pub mod robotics;
pub mod systems;

pub use project::{Metric, Project};
