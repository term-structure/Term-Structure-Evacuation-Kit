use ark_bn254::Fr;
use num_traits::Zero;

use ts_merkle_tree::{MerkleLeaf, MerkleTree, MerkleTreeWithLeaves};
use ts_poseidon::poseidon;

use super::Array;

#[derive(Clone, Copy)]
pub struct Token {
    pub avl_amt: Fr,
    pub locked_amt: Fr,
}
impl Default for Token {
    fn default() -> Self {
        Self {
            avl_amt: Fr::zero(),
            locked_amt: Fr::zero(),
        }
    }
}
impl MerkleLeaf<Fr> for Token {
    fn digest(&self) -> Result<Fr, String> {
        Ok(poseidon::<4>(&[
            Fr::from(2u64),
            self.avl_amt,
            self.locked_amt,
        ]))
    }
}
impl Token {
    pub fn income(&mut self, amt: Fr) -> Result<(), String> {
        self.avl_amt += amt;
        Ok(())
    }
    pub fn outgo(&mut self, amt: Fr) -> Result<(), String> {
        self.avl_amt -= amt;
        Ok(())
    }
    pub fn lock(&mut self, amt: Fr) -> Result<(), String> {
        self.avl_amt -= amt;
        self.locked_amt += amt;
        Ok(())
    }
    pub fn unlock(&mut self, amt: Fr) -> Result<(), String> {
        self.avl_amt += amt;
        self.locked_amt -= amt;
        Ok(())
    }
    pub fn deduct(&mut self, amt: Fr) -> Result<(), String> {
        self.locked_amt -= amt;
        Ok(())
    }
}

pub struct TokenTree<Nodes: Array<Option<Fr>>, Tokens: Array<Token>> {
    pub nodes: Nodes,
    pub actual_level: usize,
    pub tokens: Tokens,
    pub default_proof: Vec<Fr>,
}
impl<Nodes: Array<Option<Fr>>, Tokens: Array<Token>> TokenTree<Nodes, Tokens> {
    pub fn default_leaf_node() -> Vec<Fr> {
        let default_leaf_node = Token::default().digest().unwrap();
        let mut default_proof = vec![default_leaf_node];
        for i in 1..super::constants::TOKEN_TREE_HEIGHT {
            default_proof.push(Self::hash(default_proof[i - 1], default_proof[i - 1]).unwrap());
        }
        default_proof
    }
}
impl<Nodes: Array<Option<Fr>> + Default, Tokens: Array<Token> + Default> Default
    for TokenTree<Nodes, Tokens>
{
    fn default() -> Self {
        Self {
            nodes: Nodes::default(),
            actual_level: 0,
            tokens: Tokens::default(),
            default_proof: Self::default_leaf_node(),
        }
    }
}
impl<Nodes: Array<Option<Fr>> + Clone, Tokens: Array<Token> + Clone> Clone
    for TokenTree<Nodes, Tokens>
{
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            actual_level: self.actual_level,
            tokens: self.tokens.clone(),
            default_proof: self.default_proof.clone(),
        }
    }
}
impl<Nodes: Array<Option<Fr>>, Tokens: Array<Token>> MerkleTree for TokenTree<Nodes, Tokens> {
    type Node = Fr;
    fn hash(left: Self::Node, right: Self::Node) -> Result<Self::Node, String> {
        Ok(poseidon::<3>(&[left, right]))
    }
    fn idx_at(&self, idx: usize) -> Result<Option<Self::Node>, String> {
        self.nodes.get(idx)
    }
    fn idx_set(&mut self, idx: usize, node: Option<Self::Node>) -> Result<(), String> {
        self.nodes.set(idx, &node)
    }
    fn get_actual_level(&self) -> Result<usize, String> {
        Ok(self.actual_level)
    }
    fn set_actual_level(&mut self, level: usize) -> Result<(), String> {
        self.actual_level = level;
        Ok(())
    }
    fn get_default_proof(&self, idx: usize) -> Result<Self::Node, String> {
        if idx >= self.default_proof.len() {
            return Err(format!(
                "default proof index out of range: {} / {}",
                idx,
                self.default_proof.len()
            ));
        }
        Ok(self.default_proof[idx])
    }
    fn get_level(&self) -> Result<usize, String> {
        Ok(super::constants::TOKEN_TREE_HEIGHT)
    }
}
impl<Nodes: Array<Option<Fr>>, Tokens: Array<Token>> MerkleTreeWithLeaves
    for TokenTree<Nodes, Tokens>
{
    type Leaf = Token;
    fn leaf_at(&self, idx: usize) -> Result<Self::Leaf, String> {
        self.tokens.get(idx)
    }
    fn leaf_set(&mut self, idx: usize, leaf: Self::Leaf) -> Result<(), String> {
        self.tokens.set(idx, &leaf)
    }
}
