mod acc_ptr;
mod node_ptr;
mod token_ptr;
mod tsbinfo_ptr;
mod tx_ptr;
use self::{
    acc_ptr::AccPtr, node_ptr::NodePtr, token_ptr::TokenPtr, tsbinfo_ptr::TSBInfoPtr, tx_ptr::TxPtr,
};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use num_bigint::BigUint;
use num_traits::Zero;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
    usize,
};
use ts_poseidon::poseidon;
use ts_state::{
    constants::{ACCOUNT_TREE_HEIGHT, TOKEN_TREE_HEIGHT},
    AccountTree, Array as ArrayTrait, State, TokenTree, Value as ValueTrait,
};

pub struct TsFile {
    file: Arc<Mutex<File>>,
    pub latest_l1_block_id: u64,
    pub block_count: u64,
    pub tx_count: u64,
}
impl TsFile {
    pub fn open(filename: &str, l2_genesis_l1_anchor_id: Option<u64>) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .map_err(|e| e.to_string())?;
        let mut ts_file = Self {
            file: Arc::new(Mutex::new(file)),
            latest_l1_block_id: l2_genesis_l1_anchor_id
                .ok_or("cfg.l2_genesis_l1_anchor_id is required")?
                as u64,
            block_count: 1,
            tx_count: 0,
        };
        if ts_file.is_empty()? {
            ts_file.alloc_val(std::mem::size_of::<u64>() * 3)?;
            ts_file.sync()?;
            let _ = Value::alloc(&ts_file)?;
            Array::<NodePtr>::load(&ts_file, 0)?.alloc()?;
            Array::<AccPtr>::load(&ts_file, 0)?.alloc()?;
            Array::<TSBInfoPtr>::load(&ts_file, 0)?.alloc()?;
            Array::<TxPtr>::load(&ts_file, 0)?.alloc()?;
        } else {
            let mut index = 0;
            ts_file.latest_l1_block_id = load_u64(&ts_file, &mut index)?;
            ts_file.block_count = load_u64(&ts_file, &mut index)?;
            ts_file.tx_count = load_u64(&ts_file, &mut index)?;
        }
        Ok(ts_file)
    }
    pub fn sync(&self) -> Result<(), String> {
        let mut index = 0;
        write_u64(&self, &mut index, self.latest_l1_block_id)?;
        write_u64(&self, &mut index, self.block_count)?;
        write_u64(&self, &mut index, self.tx_count)?;
        self.file
            .lock()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn close(self) -> Result<(), String> {
        self.sync()
    }
    pub fn to_state(&self) -> Result<StateInstance, String> {
        let (ts_root, account_tree_nodes, accounts, tsb_infos, txs) = if !self.is_empty()? {
            let ts_root: Value = Value::load(self, 24)?;
            let account_tree_nodes: Array<NodePtr> = Array::load(&self, 56)?;
            let accounts: Array<AccPtr> = Array::load(&self, 96)?;
            let tsb_infos: Array<TSBInfoPtr> = Array::load(&self, 280)?;
            let txs: Array<TxPtr> = Array::load(&self, 352)?;
            (ts_root, account_tree_nodes, accounts, tsb_infos, txs)
        } else {
            return Err("unreachable".to_string());
        };

        let token_default_leaf_node = TokenTree::<TokenTreeNodes, Tokens>::default_leaf_node();
        let default_token_root =
            poseidon::<3>(&[token_default_leaf_node[TOKEN_TREE_HEIGHT - 1]; 2]);
        let default_leaf_node =
            poseidon::<5>(&[Fr::from(3u64), Fr::zero(), Fr::zero(), default_token_root]);
        let mut default_proof = vec![default_leaf_node];
        for i in 1..ACCOUNT_TREE_HEIGHT {
            default_proof.push(poseidon::<3>(&[default_proof[i - 1], default_proof[i - 1]]));
        }
        let actual_level = match &accounts {
            Array::Default { .. } => 0,
            Array::Alloced { indexes, .. } => indexes.len() - 1,
        };

        let account_tree = AccountTree {
            nodes: account_tree_nodes,
            accounts,
            actual_level,
            default_proof,
            _phantom: std::marker::PhantomData,
        };

        let state = StateInstance {
            ts_root,
            accounts: account_tree,
            tsb_infos,
            txs,
        };

        Ok(state)
    }
    fn is_empty(&self) -> Result<bool, String> {
        let file = self.file.lock().map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        Ok(metadata.len() == 0)
    }
    fn read<const LEN: usize>(&self, index: usize) -> Result<[u8; LEN], String> {
        let mut file = self.file.lock().map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(index as u64))
            .map_err(|e| e.to_string())?;
        let mut buf = [0u8; LEN];
        file.read(&mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    }
    fn write(&self, index: usize, buf: &[u8], len: usize) -> Result<(), String> {
        let mut file = self.file.lock().map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(index as u64))
            .map_err(|e| e.to_string())?;
        let mut buf = buf.to_vec();
        buf.resize(len, 0);
        file.write(&buf).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn alloc_val(&self, size: usize) -> Result<usize, String> {
        let mut file = self.file.lock().map_err(|e| e.to_string())?;
        let index = file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())? as usize;
        file.set_len((index + size) as u64)
            .map_err(|e| e.to_string())?;
        Ok(index)
    }
    fn alloc_arr(&self, size: usize, len: usize) -> Result<usize, String> {
        let mut file = self.file.lock().map_err(|e| e.to_string())?;
        let index = file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())? as usize;
        file.set_len((index + std::mem::size_of::<u64>() + size * len) as u64)
            .map_err(|e| e.to_string())?;
        Ok(index)
    }
    pub fn perform_with_file(
        filename: &str,
        l2_genesis_l1_anchor_id: Option<u64>,
        mut callbackfn: impl FnMut(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut ts_file = Self::open(
            filename,
            match l2_genesis_l1_anchor_id {
                Some(id) => Some(id - 1),
                None => None,
            },
        )?;
        callbackfn(&mut ts_file)?;
        ts_file.close()?;
        Ok(())
    }
}

pub trait ElemPtr<'a> {
    const SIZE: usize;
    type Elem;
    fn new(file: &'a TsFile, index: usize) -> Self;
    fn read(&self) -> Result<Self::Elem, String>;
    fn write(&self, elem: &Self::Elem) -> Result<(), String>;
    fn default(file: &'a TsFile) -> Self::Elem;
}
pub enum Array<'a, Ptr: ElemPtr<'a>> {
    Default {
        file: &'a TsFile,
    },
    Alloced {
        file: &'a TsFile,
        indexes: Vec<usize>,
        head: usize,
        _phantom: std::marker::PhantomData<Ptr>,
    },
}
impl<'a, Ptr: ElemPtr<'a>> Array<'a, Ptr> {
    pub fn load(file: &'a TsFile, head: usize) -> Result<Self, String> {
        if head == 0 {
            return Ok(Self::Default { file });
        }
        let mut indexes = vec![];
        let mut index = head;
        while index != 0 {
            indexes.push(index as usize);
            index = u64::from_le_bytes(file.read(index)?) as usize;
        }
        Ok(Self::Alloced {
            file,
            indexes,
            head,
            _phantom: std::marker::PhantomData,
        })
    }
    fn alloc(&mut self) -> Result<(), String> {
        Ok(match self {
            Self::Default { file } => {
                let head = file.alloc_arr(Ptr::SIZE, 1)?;
                let indexes = vec![head];
                *self = Self::Alloced {
                    file,
                    indexes,
                    head,
                    _phantom: std::marker::PhantomData,
                };
            }
            Self::Alloced { file, indexes, .. } => {
                let offset = 1 << (indexes.len() - 1);
                let index = file.alloc_arr(Ptr::SIZE, offset)? as u64;
                let last_idx = indexes.last().ok_or("unreachable")?;
                file.write(*last_idx, &index.to_le_bytes(), std::mem::size_of::<u64>())?;
                indexes.push(index as usize);
            }
        })
    }
}
impl<'a, Ptr: ElemPtr<'a>> ArrayTrait<Ptr::Elem> for Array<'a, Ptr> {
    fn get(&self, index: usize) -> Result<Ptr::Elem, String> {
        match self {
            Self::Default { file } => Ok(Ptr::default(file)),
            Self::Alloced { file, indexes, .. } => {
                let ptr = Ptr::new(
                    file,
                    if index == 0 {
                        *indexes.get(0).ok_or("invalid array struct")? + std::mem::size_of::<u64>()
                    } else {
                        let index = index as u64;
                        let leading_one_idx =
                            std::mem::size_of::<u64>() * 8 - 1 - index.leading_zeros() as usize;
                        let tmp = if leading_one_idx + 1 < indexes.len() {
                            indexes.get(leading_one_idx + 1).ok_or("unreachable")?
                        } else {
                            return Ok(Ptr::default(file));
                        };
                        tmp + std::mem::size_of::<u64>()
                            + (index - (1 << leading_one_idx)) as usize * Ptr::SIZE
                    },
                );
                ptr.read()
            }
        }
    }
    fn set(&mut self, index: usize, elem: &Ptr::Elem) -> Result<(), String> {
        match self {
            Self::Default { .. } => {
                self.alloc()?;
                return self.set(index, elem);
            }
            Self::Alloced { file, indexes, .. } => {
                let ptr = Ptr::new(
                    file,
                    if index == 0 {
                        *indexes.get(0).ok_or("invalid array struct")? + std::mem::size_of::<u64>()
                    } else {
                        let index = index as u64;
                        let leading_one_idx =
                            std::mem::size_of::<u64>() * 8 - 1 - index.leading_zeros() as usize;
                        let tmp = {
                            for _ in indexes.len()..=leading_one_idx + 1 {
                                self.alloc()?;
                            }
                            if let Self::Alloced { indexes, .. } = self {
                                indexes.get(leading_one_idx + 1).ok_or("unreachable")?
                            } else {
                                return Err("unreachable".to_string());
                            }
                        };
                        tmp + std::mem::size_of::<u64>()
                            + (index - (1 << leading_one_idx)) as usize * Ptr::SIZE
                    },
                );
                ptr.write(elem)
            }
        }
    }
}

pub struct Value<'a> {
    file: &'a TsFile,
    index: usize,
}
impl<'a> Value<'a> {
    pub fn load(file: &'a TsFile, index: usize) -> Result<Self, String> {
        Ok(Self { file, index })
    }
    pub fn alloc(file: &'a TsFile) -> Result<Self, String> {
        let index = file.alloc_val(std::mem::size_of::<Fr>())?;
        Ok(Self { file, index })
    }
}
impl<'a> ValueTrait for Value<'a> {
    fn get(&self) -> Result<Fr, String> {
        load_fr(self.file, &mut self.index.clone())
    }
    fn set(&mut self, value: &Fr) -> Result<(), String> {
        write_fr(self.file, &mut self.index.clone(), *value)
    }
}

fn load_fr(file: &TsFile, index: &mut usize) -> Result<Fr, String> {
    let fr = Fr::from_le_bytes_mod_order(&file.read::<32>(*index)?);
    *index += 32;
    Ok(fr)
}
fn load_u64(file: &TsFile, index: &mut usize) -> Result<u64, String> {
    let u64 = u64::from_le_bytes(file.read::<8>(*index)?);
    *index += 8;
    Ok(u64)
}
fn write_fr(file: &TsFile, index: &mut usize, fr: Fr) -> Result<(), String> {
    let biguint: BigUint = fr.into();
    let mut buf = biguint.to_bytes_le();
    buf.resize(32, 0);
    file.write(*index, &buf, 32)?;
    *index += 32;
    Ok(())
}
fn write_u64(file: &TsFile, index: &mut usize, u64: u64) -> Result<(), String> {
    file.write(*index, &u64.to_le_bytes(), 8)?;
    *index += 8;
    Ok(())
}

pub type AccountTreeNodes<'a> = Array<'a, NodePtr<'a>>;
pub type Accounts<'a> = Array<'a, AccPtr<'a>>;
pub type TokenTreeNodes<'a> = Array<'a, NodePtr<'a>>;
pub type Tokens<'a> = Array<'a, TokenPtr<'a>>;
pub type TSBInfos<'a> = Array<'a, TSBInfoPtr<'a>>;
pub type Txs<'a> = Array<'a, TxPtr<'a>>;
pub type StateInstance<'a> = State<
    Value<'a>,
    AccountTreeNodes<'a>,
    Accounts<'a>,
    TokenTreeNodes<'a>,
    Tokens<'a>,
    TSBInfos<'a>,
    Txs<'a>,
>;
