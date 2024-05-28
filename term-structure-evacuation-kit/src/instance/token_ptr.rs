use super::{load_fr, write_fr, ElemPtr, TsFile};
use ts_state::Token;

pub struct TokenPtr<'a> {
    file: &'a TsFile,
    index: usize,
}
impl<'a> ElemPtr<'a> for TokenPtr<'a> {
    const SIZE: usize = 64;
    type Elem = Token;
    fn new(file: &'a TsFile, index: usize) -> Self {
        Self { file, index }
    }
    fn read(&self) -> Result<Token, String> {
        let mut index = self.index;
        let avl_amt = load_fr(self.file, &mut index)?;
        let locked_amt = load_fr(self.file, &mut index)?;
        Ok(Token {
            avl_amt,
            locked_amt,
        })
    }
    fn write(&self, token: &Token) -> Result<(), String> {
        let mut index = self.index;
        write_fr(self.file, &mut index, token.avl_amt)?;
        write_fr(self.file, &mut index, token.locked_amt)?;
        Ok(())
    }
    fn default(_: &'a TsFile) -> Self::Elem {
        Self::Elem::default()
    }
}
