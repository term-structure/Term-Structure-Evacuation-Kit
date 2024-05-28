use std::collections::HashMap;

use ark_bn254::Fr;
use ark_ff::PrimeField;
use num_bigint::BigUint;
use num_traits::Zero;
use serde::{Deserialize, Serialize};

use super::Tx;

use self::converter::to_tx;

mod converter;

#[derive(Clone, Debug)]
pub struct Param {
    symbol: String,
    len: usize,
    is_fixed: bool,
}
#[derive(Clone, Debug)]
pub struct Schema {
    ops: Vec<(String, Vec<Param>)>,
}
impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Clone, Deserialize, Serialize, Debug)]
        struct HelperParam {
            #[serde(rename = "type")]
            base_type: String,
            symbol: String,
        }
        #[derive(Clone, Deserialize, Serialize, Debug)]
        struct HelperTx {
            name: String,
            params: Vec<HelperParam>,
        }
        #[derive(Clone, Deserialize, Serialize, Debug)]
        struct Helper {
            base_type: HashMap<String, usize>,
            transaction: Vec<HelperTx>,
        }
        let sechma = Helper::deserialize(deserializer)?;
        let mut ops = Vec::new();

        for tx in sechma.transaction {
            let mut tx_params = Vec::new();
            for param in tx.params {
                let len = *sechma.base_type.get(&param.base_type).unwrap();
                tx_params.push(Param {
                    symbol: param.symbol,
                    len,
                    is_fixed: param.base_type == "tx_amount" || param.base_type == "tx_ratio",
                });
            }
            ops.push((tx.name, tx_params));
        }
        Ok(Schema { ops })
    }
}
impl Schema {
    pub fn parse(&self, data: &mut &[u8]) -> Result<Tx, String> {
        let op_type = data[0];
        let (op_type, params) = &self.ops[op_type as usize];
        let mut ptr = 0;
        let mut map = HashMap::new();
        for i in params {
            let key = &i.symbol;
            let len = &i.len;
            let raw_val = Fr::from_be_bytes_mod_order(&data[ptr..ptr + len]);
            let val = match i.is_fixed {
                true => float2fix(raw_val),
                _ => raw_val,
            };
            map.insert(key.clone(), val);
            ptr += len;
        }
        if ptr % 12 != 0 {
            ptr += 12 - (ptr % 12);
        }
        *data = &data[ptr..];
        to_tx((op_type, map))
    }
}

fn float2fix(x: Fr) -> Fr {
    let x_biguint: BigUint = x.into();
    //total 40 bits, first 5 bits is exp, last 35 bits is mantissa
    let x_u64s = x_biguint.clone().to_u64_digits();

    if x_u64s.len() == 0 {
        Fr::zero()
    } else {
        let exp = (x_u64s[0] >> 35) as u32;
        let mantissa: BigUint = (x_u64s[0] & ((1 << 35) - 1)).into();
        let ten = BigUint::from(10u32);
        (mantissa * ten.pow(exp) as BigUint).into()
    }
}
