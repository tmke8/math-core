use strum_macros::IntoStaticStr;

#[derive(Debug, Clone, Copy, PartialEq, IntoStaticStr)]
pub enum Env {
    #[strum(serialize = "array")]
    Array,
    #[strum(serialize = "subarray")]
    Subarray,
    #[strum(serialize = "align")]
    Align,
    #[strum(serialize = "align*")]
    AlignStar,
    #[strum(serialize = "aligned")]
    Aligned,
    #[strum(serialize = "cases")]
    Cases,
    #[strum(serialize = "matrix")]
    Matrix,
    #[strum(serialize = "bmatrix")]
    BMatrix,
    #[strum(serialize = "Bmatrix")]
    Bmatrix,
    #[strum(serialize = "pmatrix")]
    PMatrix,
    #[strum(serialize = "vmatrix")]
    VMatrix,
    #[strum(serialize = "Vmatrix")]
    Vmatrix,
}

impl Env {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        ENVIRONMENTS.get(s).copied()
    }
}

static ENVIRONMENTS: phf::Map<&'static str, Env> = phf::phf_map! {
    "array" => Env::Array,
    "subarray" => Env::Subarray,
    "align" => Env::Align,
    "align*" => Env::AlignStar,
    "aligned" => Env::Aligned,
    "bmatrix" => Env::BMatrix,
    "Bmatrix" => Env::Bmatrix,
    "cases" => Env::Cases,
    "matrix" => Env::Matrix,
    "pmatrix" => Env::PMatrix,
    "vmatrix" => Env::VMatrix,
    "Vmatrix" => Env::Vmatrix,
};
