




pub mod conddc;
pub mod flatten;
pub mod constfold;
pub mod dce;
pub mod inline;
pub mod licm;
pub mod reserve;
pub mod ssa;

use prion_ir::Program;


pub fn passes() -> Vec<(&'static str, fn(&mut Program))> {
    vec![
        ("conddc", conddc::run),
        ("constfold", constfold::run),
        ("inline", inline::run),
        ("reserve", reserve::run),
        ("dce", dce::run),
        ("ssa", ssa::run),
        ("licm", licm::run),
        
        ("flatten", flatten::run),
        
        ("constfold2", constfold::run),
        ("dce2", dce::run),
    ]
}


pub fn run(prog: &mut Program) {
    for (name, pass) in passes() {
        pass(prog);
        let _ = name;
    }
}
