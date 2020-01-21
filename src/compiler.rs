use anyhow::Result;

pub trait Backend {
    type Target;

    fn compile(&self) -> Result<Self::Target>;
}
