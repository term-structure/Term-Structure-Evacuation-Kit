use super::{write_fr, ElemPtr, TsFile};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use num_traits::Zero;

pub struct NodePtr<'a> {
    file: &'a TsFile,
    index: usize,
}
impl<'a> ElemPtr<'a> for NodePtr<'a> {
    const SIZE: usize = 32;
    type Elem = Option<Fr>;
    fn new(file: &'a TsFile, index: usize) -> Self {
        Self { file, index }
    }
    fn read(&self) -> Result<Option<Fr>, String> {
        let arr: [u8; 32] = self.file.read(self.index)?;
        if arr == [0u8; 32] {
            Ok(None)
        } else if arr == [255u8; 32] {
            Ok(Some(Fr::zero()))
        } else {
            Ok(Some(Fr::from_le_bytes_mod_order(&arr)))
        }
    }
    fn write(&self, node: &Option<Fr>) -> Result<(), String> {
        let mut index = self.index;
        Ok(match node {
            Some(node) => {
                if node == &Fr::zero() {
                    self.file.write(index, &[255u8; 32], Self::SIZE)?;
                } else {
                    write_fr(self.file, &mut index, *node)?;
                }
            }
            None => {
                self.file.write(index, &[0u8; 32], Self::SIZE)?;
            }
        })
    }
    fn default(_: &'a TsFile) -> Self::Elem {
        Self::Elem::default()
    }
}
