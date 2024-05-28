use super::{ElemPtr, TsFile};
use ts_state::Tx;

pub struct TxPtr<'a> {
    file: &'a TsFile,
    index: usize,
}
impl<'a> ElemPtr<'a> for TxPtr<'a> {
    const SIZE: usize = 512; // 512 is std::mem::size_of::<Tx>()
    type Elem = Tx;
    fn new(file: &'a TsFile, index: usize) -> Self {
        Self { file, index }
    }
    fn read(&self) -> Result<Self::Elem, String> {
        let bytes: [u8; 512] = self.file.read(self.index)?;
        let elem = unsafe { std::mem::transmute(bytes) };
        Ok(elem)
    }
    fn write(&self, elem: &Self::Elem) -> Result<(), String> {
        let bytes: [u8; 512] = unsafe { std::mem::transmute(*elem) };
        self.file.write(self.index, &bytes, Self::SIZE)
    }
    fn default(_: &'a TsFile) -> Self::Elem {
        Tx::default()
    }
}
