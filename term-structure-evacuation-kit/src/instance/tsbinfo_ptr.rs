use super::{load_fr, load_u64, write_fr, write_u64, ElemPtr, TsFile};
use ts_state::TSBInfo;

pub struct TSBInfoPtr<'a> {
    file: &'a TsFile,
    index: usize,
}
impl<'a> ElemPtr<'a> for TSBInfoPtr<'a> {
    const SIZE: usize = 64;
    type Elem = TSBInfo;
    fn new(file: &'a TsFile, index: usize) -> Self {
        Self { file, index }
    }
    fn read(&self) -> Result<Self::Elem, String> {
        let mut index = self.index;
        let base_token_id = load_u64(self.file, &mut index)? as usize;
        let maturity = load_fr(self.file, &mut index)?;
        Ok(TSBInfo {
            base_token_id,
            maturity,
        })
    }
    fn write(&self, elem: &Self::Elem) -> Result<(), String> {
        let mut index = self.index;
        write_u64(self.file, &mut index, elem.base_token_id as u64)?;
        write_fr(self.file, &mut index, elem.maturity)?;
        Ok(())
    }
    fn default(_: &'a TsFile) -> Self::Elem {
        Self::Elem::default()
    }
}
