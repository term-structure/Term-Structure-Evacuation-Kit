use super::{
    load_fr, load_u64, write_fr, write_u64, Array, ElemPtr, TokenTreeNodes, Tokens, TsFile,
};
use ark_bn254::Fr;
use num_traits::Zero;
use ts_state::{Account, TokenTree};

pub struct AccPtr<'a> {
    file: &'a TsFile,
    index: usize,
}
impl<'a> ElemPtr<'a> for AccPtr<'a> {
    const SIZE: usize = 176;
    type Elem = Account<TokenTreeNodes<'a>, Tokens<'a>>;
    fn new(file: &'a TsFile, index: usize) -> Self {
        Self { file, index }
    }
    fn read(&self) -> Result<Self::Elem, String> {
        let mut index = self.index;
        let l2_addr = load_fr(self.file, &mut index)?;
        let nonce = load_fr(self.file, &mut index)?;
        let nodes = TokenTreeNodes::load(self.file, load_u64(self.file, &mut index)? as usize)?;
        let leaves = Tokens::load(self.file, load_u64(self.file, &mut index)? as usize)?;
        let actual_level = match &leaves {
            Array::Default { .. } => 0,
            Array::Alloced { indexes, .. } => indexes.len() - 1,
        };
        let default_proof = TokenTree::<TokenTreeNodes, Tokens>::default_leaf_node();
        let token_tree = TokenTree {
            nodes,
            actual_level,
            tokens: leaves,
            default_proof,
        };
        Ok(Account {
            l2_addr,
            nonce,
            tokens: token_tree,
        })
    }
    fn write(&self, elem: &Self::Elem) -> Result<(), String> {
        fn f<'a, T: ElemPtr<'a>>(
            file: &'a TsFile,
            index: &mut usize,
            arr: &Array<'a, T>,
        ) -> Result<(), String> {
            match arr {
                Array::Default { .. } => {
                    write_u64(file, index, 0)?;
                }
                Array::Alloced { head, .. } => {
                    write_u64(file, index, *head as u64)?;
                }
            }
            Ok(())
        }
        let mut index = self.index;
        write_fr(self.file, &mut index, elem.l2_addr)?;
        write_fr(self.file, &mut index, elem.nonce)?;
        f(self.file, &mut index, &elem.tokens.nodes)?;
        f(self.file, &mut index, &elem.tokens.tokens)?;
        Ok(())
    }
    fn default(file: &'a TsFile) -> Self::Elem {
        Account {
            l2_addr: Fr::zero(),
            nonce: Fr::zero(),
            tokens: TokenTree {
                nodes: TokenTreeNodes::Default { file },
                actual_level: 0,
                tokens: Tokens::Default { file },
                default_proof: TokenTree::<TokenTreeNodes, Tokens>::default_leaf_node(),
            },
        }
    }
}
