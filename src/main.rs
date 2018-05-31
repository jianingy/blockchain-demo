#[macro_use] extern crate failure;
#[macro_use] extern crate nickel;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

extern crate chrono;
extern crate reqwest;
extern crate serde;
extern crate sha2;


use nickel::{Nickel, HttpRouter, Request, Response, MiddlewareResult};
use std::sync::{Arc, Mutex};
use std::io::Read;
mod blockchain;

fn enable_json<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    res.headers_mut().set_raw("Content-Type", vec![b"application/json; charset=UTF-8".to_vec()]);
    res.next_middleware()
}

fn main() {
    let mut server = Nickel::new();
    let blockchain = Arc::new(Mutex::new(blockchain::Blockchain::new()));
    server.utilize(enable_json);

    let chain = blockchain.clone();
    server.get("/chain", middleware! {
        let chain = chain.lock().unwrap();
        serde_json::to_string(&*chain).unwrap()
    });

    let chain = blockchain.clone();
    server.get("/mine", middleware! {
        let mut chain = chain.lock().unwrap();
        let hash = chain.mine().unwrap();
        serde_json::to_string(&hash).unwrap()
    });

    let chain = blockchain.clone();
    server.post("/transactions/new", middleware! {
        |req, _resp|
        let mut chain = chain.lock().unwrap();
        let mut body = vec![];
        req.origin.read_to_end(&mut body).unwrap();
        let trx: blockchain::Transaction =
            serde_json::from_str(&String::from_utf8_lossy(&body)).unwrap();
        chain.new_transaction(&trx.sender, &trx.recipient, trx.amount);
        serde_json::to_string(&*chain.transactions).unwrap()
    });

    let chain = blockchain.clone();
    server.post("/nodes/register", middleware! {
        |req, _resp|
        let mut chain = chain.lock().unwrap();
        let mut body = vec![];
        req.origin.read_to_end(&mut body).unwrap();
        let nodes: Vec<String> =
            serde_json::from_str(&String::from_utf8_lossy(&body)).unwrap();
        for node in nodes.iter() {
            chain.register_node(node);
        }
        serde_json::to_string(&nodes).unwrap()
    });

    let chain = blockchain.clone();
    server.post("/nodes/resolve", middleware! {
        let mut chain = chain.lock().unwrap();
        match chain.resolve_conflicts() {
            Ok(true) => json!({
                "new_chain": serde_json::to_string(&*chain).unwrap(),
            }).to_string(),
            Ok(false) => json!({
                "chain":serde_json::to_string(&*chain).unwrap(),
            }).to_string(),
            _ => json!({
                "message": "cannot resolve"
            }).to_string()
        }
    });

    server.listen("127.0.0.1:8000").unwrap();
}
