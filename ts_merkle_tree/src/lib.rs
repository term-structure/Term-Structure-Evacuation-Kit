use serde::{Deserialize, Serialize};

fn log2_of_usize(mut n: usize) -> usize {
    let mut log2 = 0;
    while n > 1 {
        log2 += 1;
        n >>= 1;
    }
    log2
}
fn lv_order_idx_to_idx(level: usize, idx: usize) -> usize {
    if idx == 0 {
        panic!("idx 0 is not a valid index");
    }
    let depth = log2_of_usize(idx);
    let position = idx - (1 << depth);

    let n = level - depth;
    let start = (1 << n) - 1;

    let offset = 1 << (n + 1);

    let new_idx = start + position * offset;
    new_idx
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleVerifyPrf<Node: Clone + Copy> {
    pub ori_root: Node,
    pub new_root: Node,
    pub ori_leaf_node: Node,
    pub new_leaf_node: Node,
    pub leaf_id: usize,
    pub proof: Vec<Node>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleVerifyPrfWithLeafData<Node: Clone + Copy, Leaf: MerkleLeaf<Node>> {
    pub merkle_prf: MerkleVerifyPrf<Node>,
    pub leaf: Leaf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleUpdatePrf<Node: Clone + Copy> {
    pub ori_root: Node,
    pub new_root: Node,
    pub ori_leaf_node: Node,
    pub new_leaf_node: Node,
    pub leaf_id: usize,
    pub proof: Vec<Node>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleUpdatePrfWithLeafData<Node: Clone + Copy, Leaf: MerkleLeaf<Node>> {
    pub merkle_prf: MerkleUpdatePrf<Node>,
    pub ori_leaf: Leaf,
    pub new_leaf: Leaf,
}
pub trait MerkleTree {
    type Node: Clone + Copy;
    fn hash(left: Self::Node, right: Self::Node) -> Result<Self::Node, String>;
    fn idx_at(&self, idx: usize) -> Result<Option<Self::Node>, String>;
    fn idx_set(&mut self, idx: usize, node: Option<Self::Node>) -> Result<(), String>;
    fn get_actual_level(&self) -> Result<usize, String>;
    fn set_actual_level(&mut self, level: usize) -> Result<(), String>;
    fn get_default_proof(&self, idx: usize) -> Result<Self::Node, String>;
    fn get_level(&self) -> Result<usize, String>;

    fn get_root(&self) -> Result<Self::Node, String> {
        let mut root = self.lv_order_at(1 << (self.get_level()? - self.get_actual_level()?))?;
        for i in self.get_actual_level()?..self.get_level()? {
            root = Self::hash(root, self.get_default_proof(i)?)?;
        }
        Ok(root)
    }
    fn lv_order_at(&self, idx: usize) -> Result<Self::Node, String> {
        match self.idx_at(lv_order_idx_to_idx(self.get_level()?, idx))? {
            Some(node) => Ok(node),
            None => self.get_default_proof(self.get_level()? - log2_of_usize(idx)),
        }
    }
    fn lv_order_set(&mut self, idx: usize, node: Self::Node) -> Result<(), String> {
        self.idx_set(lv_order_idx_to_idx(self.get_level()?, idx), Some(node))?;
        Ok(())
    }
    fn leaf_id_at(&self, leaf_id: usize) -> Result<Self::Node, String> {
        match self.idx_at(leaf_id * 2)? {
            Some(node) => Ok(node),
            None => self.get_default_proof(0),
        }
    }
    fn leaf_id_set(&mut self, leaf_id: usize, node: Self::Node) -> Result<(), String> {
        self.idx_set(leaf_id * 2, Some(node))?;
        Ok(())
    }
    fn verify_leaf_node(&self, leaf_id: usize) -> Result<MerkleVerifyPrf<Self::Node>, String> {
        let mut proof = vec![];
        let mut idx = leaf_id + (1 << self.get_level()?);
        for _ in 0..self.get_actual_level()? {
            proof.push(match idx & 1 == 0 {
                true => self.lv_order_at(idx + 1)?,
                false => self.lv_order_at(idx - 1)?,
            });
            idx >>= 1;
        }
        for i in self.get_actual_level()?..self.get_level()? {
            proof.push(self.get_default_proof(i)?);
            idx >>= 1;
        }
        Ok(MerkleVerifyPrf {
            ori_root: self.get_root()?,
            new_root: self.get_root()?,
            ori_leaf_node: self.leaf_id_at(leaf_id)?,
            new_leaf_node: self.leaf_id_at(leaf_id)?,
            leaf_id,
            proof,
        })
    }
    fn update_leaf_node(
        &mut self,
        leaf_id: usize,
        node: Self::Node,
    ) -> Result<MerkleUpdatePrf<Self::Node>, String> {
        if log2_of_usize(leaf_id) > self.get_level()? {
            panic!("leaf_id is too large");
        }
        if log2_of_usize(leaf_id) + 1 > self.get_actual_level()? {
            let new_actual_level = log2_of_usize(leaf_id) + 1;
            for i in self.get_actual_level()?..new_actual_level {
                let idx = 1 << (self.get_level()? - i);
                self.lv_order_set(
                    idx >> 1,
                    Self::hash(self.lv_order_at(idx)?, self.get_default_proof(i)?)?,
                )?;
            }
            self.set_actual_level(new_actual_level)?;
        }

        let mut idx = leaf_id + (1 << self.get_level()?);
        let mut proof = vec![];
        let ori_root = self.get_root()?;
        let ori_leaf_node = self.leaf_id_at(leaf_id)?;
        self.lv_order_set(idx, node)?;
        for _ in 0..self.get_actual_level()? {
            let node = self.lv_order_at(idx)?;
            match idx & 1 == 0 {
                true => {
                    let brother = self.lv_order_at(idx + 1)?;
                    self.lv_order_set(idx >> 1, Self::hash(node, brother)?)?;
                    proof.push(brother);
                }
                false => {
                    let brother = self.lv_order_at(idx - 1)?;
                    self.lv_order_set(idx >> 1, Self::hash(brother, node)?)?;
                    proof.push(brother);
                }
            }
            idx >>= 1;
        }
        let mut node = self.lv_order_at(idx).unwrap();
        for i in self.get_actual_level()?..self.get_level()? {
            let brother = self.get_default_proof(i)?;
            match idx & 1 == 0 {
                true => node = Self::hash(node, brother)?,
                false => node = Self::hash(brother, node)?,
            }
            proof.push(brother);
            idx >>= 1;
        }
        Ok(MerkleUpdatePrf {
            ori_root,
            new_root: self.get_root()?,
            ori_leaf_node,
            new_leaf_node: self.leaf_id_at(leaf_id)?,
            leaf_id,
            proof,
        })
    }
}
pub trait MerkleLeaf<Node: Clone + Copy> {
    fn digest(&self) -> Result<Node, String>;
}

pub trait MerkleTreeWithLeaves: MerkleTree {
    type Leaf: MerkleLeaf<Self::Node>;
    fn leaf_at(&self, idx: usize) -> Result<Self::Leaf, String>;
    fn leaf_set(&mut self, idx: usize, leaf: Self::Leaf) -> Result<(), String>;
    fn verify_leaf(
        &self,
        idx: usize,
    ) -> Result<MerkleVerifyPrfWithLeafData<Self::Node, Self::Leaf>, String> {
        let merkle_prf = self.verify_leaf_node(idx)?;
        let leaf = self.leaf_at(idx)?;
        Ok(MerkleVerifyPrfWithLeafData { merkle_prf, leaf })
    }
    fn update_leaf(
        &mut self,
        idx: usize,
        leaf: Self::Leaf,
    ) -> Result<MerkleUpdatePrfWithLeafData<Self::Node, Self::Leaf>, String> {
        let digest = leaf.digest();
        let ori_leaf = self.leaf_at(idx)?;
        let merkle_prf = self.update_leaf_node(idx, digest?)?;
        self.leaf_set(idx, leaf)?;
        let new_leaf = self.leaf_at(idx)?;
        let prf = MerkleUpdatePrfWithLeafData {
            merkle_prf,
            ori_leaf,
            new_leaf,
        };
        Ok(prf)
    }
    fn update(
        &mut self,
        idx: u64,
        f: impl FnOnce(&mut Self::Leaf) -> Result<(), String>,
    ) -> Result<MerkleUpdatePrfWithLeafData<Self::Node, Self::Leaf>, String> {
        let mut leaf = self.leaf_at(idx as usize)?;
        f(&mut leaf)?;
        self.update_leaf(idx as usize, leaf)
    }
}
