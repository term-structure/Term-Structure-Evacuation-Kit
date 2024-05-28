use super::{
    token::{Token, TokenTree},
    Array,
};
use ark_bn254::Fr;
use num_traits::{One, Zero};
use ts_merkle_tree::{MerkleLeaf, MerkleTree, MerkleTreeWithLeaves};
use ts_poseidon::poseidon;

pub struct Account<TokenTreeNodes: Array<Option<Fr>>, Tokens: Array<Token>> {
    pub l2_addr: Fr,
    pub nonce: Fr,
    pub tokens: TokenTree<TokenTreeNodes, Tokens>,
}
impl<TokenTreeNodes: Array<Option<Fr>> + Default, Tokens: Array<Token> + Default> Default
    for Account<TokenTreeNodes, Tokens>
{
    fn default() -> Self {
        Self {
            l2_addr: Fr::zero(),
            nonce: Fr::zero(),
            tokens: TokenTree::default(),
        }
    }
}
impl<TokenTreeNodes: Array<Option<Fr>> + Clone, Tokens: Array<Token> + Clone> Clone
    for Account<TokenTreeNodes, Tokens>
{
    fn clone(&self) -> Self {
        Self {
            l2_addr: self.l2_addr.clone(),
            nonce: self.nonce.clone(),
            tokens: self.tokens.clone(),
        }
    }
}
impl<TokenTreeNodes: Array<Option<Fr>>, Tokens: Array<Token>> MerkleLeaf<Fr>
    for Account<TokenTreeNodes, Tokens>
{
    fn digest(&self) -> Result<Fr, String> {
        Ok(poseidon::<5>(&[
            Fr::from(3u64),
            self.l2_addr,
            self.nonce,
            self.tokens.get_root()?,
        ]))
    }
}
impl<TokenTreeNodes: Array<Option<Fr>>, Tokens: Array<Token>> Account<TokenTreeNodes, Tokens> {
    pub fn set_l2_addr(&mut self, l2_addr: Fr) -> Result<(), String> {
        self.l2_addr = l2_addr;
        Ok(())
    }
    pub fn increase_nonce(&mut self) -> Result<(), String> {
        self.nonce += Fr::one();
        Ok(())
    }
    pub fn income(&mut self, token_id: u64, amt: Fr) -> Result<(), String> {
        self.tokens
            .update(token_id, |token| Ok(token.income(amt)?))?;
        Ok(())
    }
    pub fn outgo(&mut self, token_id: u64, amt: Fr) -> Result<(), String> {
        self.tokens
            .update(token_id, |token| Ok(token.outgo(amt)?))?;
        Ok(())
    }
    pub fn lock(&mut self, token_id: u64, amt: Fr) -> Result<(), String> {
        self.tokens.update(token_id, |token| Ok(token.lock(amt)?))?;
        Ok(())
    }
    pub fn unlock(&mut self, token_id: u64, amt: Fr) -> Result<(), String> {
        self.tokens
            .update(token_id, |token| Ok(token.unlock(amt)?))?;
        Ok(())
    }
    pub fn deduct(&mut self, token_id: u64, amt: Fr) -> Result<(), String> {
        self.tokens
            .update(token_id, |token| Ok(token.deduct(amt)?))?;
        Ok(())
    }
}

pub struct AccountTree<
    AccountTreeNodes: Array<Option<Fr>>,
    Accounts: Array<Account<TokenTreeNodes, Tokens>>,
    TokenTreeNodes: Array<Option<Fr>>,
    Tokens: Array<Token>,
> {
    pub nodes: AccountTreeNodes,
    pub actual_level: usize,
    pub accounts: Accounts,
    pub default_proof: Vec<Fr>,
    pub _phantom: std::marker::PhantomData<(TokenTreeNodes, Tokens)>,
}
impl<
        AccountTreeNodes: Array<Option<Fr>> + Default,
        Accounts: Array<Account<TokenTreeNodes, Tokens>> + Default,
        TokenTreeNodes: Array<Option<Fr>> + Default,
        Tokens: Array<Token> + Default,
    > Default for AccountTree<AccountTreeNodes, Accounts, TokenTreeNodes, Tokens>
{
    fn default() -> Self {
        let default_leaf = Account::<TokenTreeNodes, Tokens>::default();
        let default_leaf_node = default_leaf.digest().unwrap();
        let mut default_proof = vec![default_leaf_node];
        for i in 1..super::constants::ACCOUNT_TREE_HEIGHT {
            default_proof.push(Self::hash(default_proof[i - 1], default_proof[i - 1]).unwrap());
        }
        Self {
            nodes: AccountTreeNodes::default(),
            actual_level: 0,
            accounts: Accounts::default(),
            default_proof,
            _phantom: std::marker::PhantomData,
        }
    }
}
impl<
        AccountTreeNodes: Array<Option<Fr>>,
        Accounts: Array<Account<TokenTreeNodes, Tokens>>,
        TokenTreeNodes: Array<Option<Fr>>,
        Tokens: Array<Token>,
    > MerkleTree for AccountTree<AccountTreeNodes, Accounts, TokenTreeNodes, Tokens>
{
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
        Ok(super::constants::ACCOUNT_TREE_HEIGHT)
    }
}
impl<
        AccountTreeNodes: Array<Option<Fr>>,
        Accounts: Array<Account<TokenTreeNodes, Tokens>>,
        TokenTreeNodes: Array<Option<Fr>>,
        Tokens: Array<Token>,
    > MerkleTreeWithLeaves for AccountTree<AccountTreeNodes, Accounts, TokenTreeNodes, Tokens>
{
    type Leaf = Account<TokenTreeNodes, Tokens>;
    fn leaf_at(&self, idx: usize) -> Result<Self::Leaf, String> {
        self.accounts.get(idx)
    }
    fn leaf_set(&mut self, idx: usize, leaf: Self::Leaf) -> Result<(), String> {
        self.accounts.set(idx, &leaf)
    }
}
