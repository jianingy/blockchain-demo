#[macro_use] extern crate serde_derive;
extern crate actix;
extern crate actix_web;
extern crate bytes;
extern crate chrono;
extern crate failure;
extern crate futures;
extern crate listenfd;
extern crate serde;
extern crate serde_json;
extern crate sha2;


use actix_web::{server, App, Json, State, HttpRequest, HttpResponse, http::Method};
use chrono::prelude::*;
use failure::Error;
use listenfd::ListenFd;
use sha2::{Sha256, Digest};
use std::mem;
use std::sync::{Arc, Mutex};
use serde::ser::{Serialize, Serializer};

type BlockProof = u64;
type ChainType = Arc<Mutex<Blockchain>>;

#[derive(Clone, Debug)]
struct BlockHash([u8; 32]);

impl Serialize for BlockHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hash: Vec<String> = self.0.iter().map(|x| format!("{:02x}", x)).collect();
        serializer.serialize_str(&hash.join(""))
    }
}

#[derive(Serialize, Clone, Debug)]
struct Block {
    index: u64,
    timestamp: i64,
    transactions: Vec<Transaction>,
    proof: BlockProof,
    previous_hash: BlockHash,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Transaction {
    sender: String,
    recipient: String,
    amount: f32,
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

#[derive(Serialize, Debug, Clone)]
pub struct Blockchain {
    chain: Vec<Block>,
    transactions: Vec<Transaction>,
}

impl Blockchain {

    fn new() -> Blockchain {
        let mut chain = Blockchain {
            chain: Vec::new(),
            transactions: Vec::new(),
        };
        chain.new_block(&BlockHash([0; 32]), 100);
        return chain;
    }

    fn new_block(&mut self, previous_hash: &BlockHash, proof: BlockProof) {
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

    fn new_transaction(&mut self, sender: &str, recipient: &str, amount: f32) -> u64 {
        self.transactions.push(Transaction::new(sender, recipient, amount));
        if let Some(block) = self.last_block_mut() {
            block.index += 1;
            return block.index;
        } else {
            panic!("no available block");
        }
    }

    fn last_block_mut(&mut self) -> Option<&mut Block> {
        self.chain.last_mut()
    }

    fn last_block(&self) -> Option<&Block> {
        self.chain.last()
    }

    fn hash(block: &Block) -> Result<BlockHash, Error> {
        let mut hash = [0; 32];
        let string = serde_json::to_string(block)?;
        let digest = Sha256::digest(string.as_bytes());
        hash.copy_from_slice(digest.as_slice());
        println!("found hash {:?}", hash);
        Ok(BlockHash(hash))
    }

    fn proof_of_work(&self, last_proof: BlockProof) -> BlockProof {
        let mut proof: BlockProof = 0;
        while !self.validate_proof(last_proof, proof) {
            proof += 1;
        }
        println!("found valid proof {:?}", proof);
        return proof;
    }

    fn validate_proof(&self, last_proof: BlockProof, proof: BlockProof) -> bool {
        let guess = (last_proof << 32) | proof;
        let hash = Sha256::digest(format!("{:x}", guess).as_bytes());
        return hash[0..2] == [0, 0];
    }

    fn len(&self) -> usize {
        self.chain.len()
    }

}

fn mine(req: HttpRequest<ChainType>) -> HttpResponse {
    let ref mut chain = *req.state().lock().unwrap();
    let (proof, previous_hash) = if let Some(last_block) = chain.last_block() {
        let last_proof = last_block.proof;
        let proof = chain.proof_of_work(last_proof);
        if let Ok(previous_hash) = Blockchain::hash(&last_block) {
            (proof, previous_hash)
        } else {
            return HttpResponse::Ok().body("cannot calculate previous block hash");
        }
    } else {
        panic!("genesis block not found");
    };
    chain.new_block(&previous_hash, proof);
    HttpResponse::Ok().json(previous_hash)
}

fn new_transactions(data: (Json<Transaction>, State<ChainType>)) -> HttpResponse {
    let (trx, chain_lock) = data;
    let mut chain = chain_lock.lock().unwrap();
    chain.new_transaction(&trx.sender, &trx.recipient, trx.amount);
    HttpResponse::Ok().json(&chain.transactions)
}

fn full_chain(req: HttpRequest<ChainType>) -> HttpResponse {
    #[derive(Serialize)]
    struct ChainInfo<'a> {
        chain: &'a Vec<Block>,
        transactions: &'a Vec<Transaction>,
        length: usize,
    }
    let chain = req.state().lock().unwrap();
    HttpResponse::Ok().json(ChainInfo {
        chain: &chain.chain,
        transactions: &chain.transactions,
        length: chain.len(),
    })
}

fn main() {
    let mut listenfd = ListenFd::from_env();
    let chain = Arc::new(Mutex::new(Blockchain::new()));
    let http_server = server::new(move || {
        let chain = chain.clone();
        App::with_state(chain)
            .route("/mine", Method::GET, mine)
            .route("/chain", Method::GET, full_chain)
            .route("/transactions/new", Method::POST, new_transactions)
    }

    );
    let http_server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        http_server.listen(l)
    } else {
        http_server.bind("127.0.0.1:8000").unwrap()
    };
    http_server.run();
}
