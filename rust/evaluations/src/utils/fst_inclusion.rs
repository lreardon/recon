use std::sync::Arc;

pub struct NullaryAndFst<'a> {
    pub nullary: &'a String,
    pub fst: Arc<fst::Map<Vec<u8>>>,
}

pub fn fst_contains_nullary(params: NullaryAndFst) -> bool {
    return params.fst.contains_key(params.nullary);
}
