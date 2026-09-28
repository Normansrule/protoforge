//! Times every module's `metrics()` on the Iris project (cold and warm).
use forge_core::project::Project;
use std::time::Instant;
type F = fn(&Project) -> Vec<forge_core::Metric>;
fn main() {
    let p = Project::iris();
    let mods: &[(&str, F)] = &[
        ("power", forge_core::power::metrics), ("optics", forge_core::optics::metrics),
        ("circuit", forge_core::circuit::metrics), ("dsp", forge_core::dsp::metrics),
        ("chip", forge_core::chip::metrics), ("control", forge_core::control::metrics),
        ("robotics", forge_core::robotics::metrics), ("rl", forge_core::rl::metrics),
        ("cad", forge_core::cad::metrics), ("print", forge_core::print::metrics),
        ("pcb", forge_core::pcb::metrics), ("molding", forge_core::molding::metrics),
        ("mfg", forge_core::mfg::metrics),
    ];
    for pass in ["cold", "warm"] {
        let mut tot = 0.0;
        for (n, f) in mods {
            let t = Instant::now();
            let _ = f(&p);
            let ms = t.elapsed().as_secs_f64() * 1e3;
            tot += ms;
            println!("{pass:5} {n:10} {ms:8.2} ms");
        }
        println!("{pass:5} total      {tot:8.2} ms\n");
    }
}
