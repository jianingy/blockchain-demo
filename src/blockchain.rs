use chrono::prelude::*;
use failure::Error;
use reqwest;
use serde::ser::{Serialize, Serializer};
use serde_json;
use sha2::{Sha256, Digest};
use std::mem;

type BlockProof = u64;

#[derive(Debug, Fail)]
pub enum BlockchainError {
    #[fail(display = "cannot calculate hash value")]
    HashFailed,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BlockHash([u8; 32]);

/*
impl Serialize for BlockHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hash: Vec<String> = self.0.iter().map(|x| format!("{:02x}", x)).collect();
        serializer.serialize_str(&hash.join(""))
    }
}
*/

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    index: u64,
    timestamp: i64,
    transactions: Vec<Transaction>,
    proof: BlockProof,
    previous_hash: BlockHash,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    pub amount: f32,
}

impl Transaction {
    fn new(sender: &str, recipient: &str, amount: f32) -> Transaction {
        Transaction {
            sender: sender.to_owned(),
            recipient: recipient.to_owned(),
            amount: amount,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub transactions: Vec<Transaction>,
    pub nodes: Vec<String>,
}

impl Blockchain {

    pub fn new() -> Blockchain {
        let mut chain = Blockchain {
            chain: Vec::new(),
            transactions: Vec::new(),
            nodes: Vec::new(),
        };
        chain.new_block(&BlockHash([0; 32]), 100);
        return chain;
    }

    pub fn new_block(&mut self, previous_hash: &BlockHash, proof: BlockProof) {
        let trx = mem::replace(&mut self.transactions, Vec::new());
        let block = Block {
            index: self.chain.len() as u64 + 1,
            timestamp: Utc::now().timestamp_millis(),
            transactions: trx,
            proof: proof,
            previous_hash: previous_hash.clone()
        };
        self.chain.push(block);
    }

    pub fn new_transaction(&mut self, sender: &str, recipient: &str, amount: f32) -> u64 {
        self.transactions.push(Transaction::new(sender, recipient, amount));
        if let Some(block) = self.last_block_mut() {
            block.index += 1;
            return block.index;
        } else {
            panic!("no available block");
        }
    }

    pub fn mine(&mut self) -> Result<BlockHash, BlockchainError> {
        let (proof, previous_hash) = if let Some(last_block) = self.last_block() {
            let last_proof = last_block.proof;
            let proof = self.proof_of_work(last_proof);
            if let Ok(previous_hash) = Blockchain::hash(&last_block) {
                (proof, previous_hash)
            } else {
                return Err(BlockchainError::HashFailed);
            }
        } else {
            panic!("genesis block not found");
        };
        self.new_block(&previous_hash, proof);
        Ok(previous_hash)
    }

    pub fn last_block_mut(&mut self) -> Option<&mut Block> {
        self.chain.last_mut()
    }

    pub fn last_block(&self) -> Option<&Block> {
        self.chain.last()
    }

    pub fn hash(block: &Block) -> Result<BlockHash, Error> {
        let mut hash = [0; 32];
        let string = serde_json::to_string(block)?;
        let digest = Sha256::digest(string.as_bytes());
        hash.copy_from_slice(digest.as_slice());
        println!("found hash {:?}", hash);
        Ok(BlockHash(hash))
    }

    pub fn proof_of_work(&self, last_proof: BlockProof) -> BlockProof {
        let mut proof: BlockProof = 0;
        while !self.valid_proof(last_proof, proof) {
            proof += 1;
        }
        println!("found valid proof {:?}", proof);
        return proof;
    }

    pub fn valid_proof(&self, last_proof: BlockProof, proof: BlockProof) -> bool {
        let guess = (last_proof << 32) | proof;
        let hash = Sha256::digest(format!("{:x}", guess).as_bytes());
        return hash[0..2] == [0, 0];
    }

    pub fn len(&self) -> usize {
        self.chain.len()
    }

    pub fn register_node(&mut self, node: &str) {
        self.nodes.push(node.to_owned())
    }

    pub fn valid_chain(&self, chain: &Vec<Block>) -> Result<bool, Error> {
        let mut last_block = &chain[0];
        for block in chain.iter().skip(1) {
            println!("{:?}\n{:?}\n--------", last_block, block);
            if block.previous_hash != Blockchain::hash(last_block)? {
                return Ok(false);
            }

            if !self.valid_proof(last_block.proof, block.proof) {
                return Ok(false);
            }

            last_block = block;
        }
        return Ok(true);
    }

    pub fn resolve_conflicts(&mut self) -> Result<bool, Error> {
        let mut flag = false;
        for node in self.nodes.iter() {
            let api = format!("{}/chain", node);
            let text = reqwest::get(&api)?.text()?;
            match serde_json::from_str::<Blockchain>(&text) {
                Ok(chain) => {
                    if chain.len() > self.len() {
                        match self.valid_chain(&chain.chain) {
                            Ok(true) => {
                                let _chain = mem::replace(&mut self.chain, chain.chain);
                                flag = true;
                            },
                            _ => continue
                        };
                    }
                },
                _ => continue
            }
        }
        return Ok(flag);
    }

}
