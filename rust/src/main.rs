#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;

// Node access params
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// You can use calls not provided in RPC lib API using the generic `call` function.
// An example of using the `send` RPC call, which doesn't have exposed API.
// You can also use serde_json `Deserialize` derivation to capture the returned json result.
fn send(rpc: &Client, addr: &str) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{addr : 100 }]), // recipient address
        json!(null),            // conf target
        json!(null),            // estimate mode
        json!(null),            // fee rate in sats/vb
        json!(null),            // Empty option object
    ];

    #[derive(Deserialize)]
    struct SendResult {
        complete: bool,
        txid: String,
    }
    let send_result = rpc.call::<SendResult>("send", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}

fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to Bitcoin Core RPC
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Get blockchain info
    let blockchain_info = rpc.get_blockchain_info()?;
    println!("Blockchain Info: {:?}", blockchain_info);

    // ___________________________________________________________________________________
    // Create/Load the wallets, named 'Miner' and 'Trader'. Have logic to optionally create/load them if they do not exist or not loaded already.
    // ___________________________________________________________________________________

    let miner_wallet_name = "Miner";
    let trader_wallet_name = "Trader";

    // Ensure Miner wallet is loaded
    if !rpc.list_wallets()?.contains(&miner_wallet_name.to_string()) {
        match rpc.load_wallet(miner_wallet_name) {
            Ok(_) => println!("Loaded existing Miner wallet"),
            Err(_) => {
                match rpc.create_wallet(miner_wallet_name, None, None, None, None) {
                    Ok(_) => println!("Created new Miner wallet"),
                    Err(_) => {
                        // Try loading again - wallet exists but wasn't loaded
                        rpc.load_wallet(miner_wallet_name)?;
                        println!("Loaded existing Miner wallet on retry");
                    }
                }
            }
        }
    }

    // Ensure Trader wallet is loaded
    if !rpc
        .list_wallets()?
        .contains(&trader_wallet_name.to_string())
    {
        match rpc.load_wallet(trader_wallet_name) {
            Ok(_) => println!("Loaded existing Trader wallet"),
            Err(_) => match rpc.create_wallet(trader_wallet_name, None, None, None, None) {
                Ok(_) => println!("Created new Trader wallet"),
                Err(_) => {
                    rpc.load_wallet(trader_wallet_name)?;
                    println!("Loaded existing Trader wallet on retry");
                }
            },
        }
    }

    // ___________________________________________________________________________________
    // Generate spendable balances in the Miner wallet. How many blocks needs to be mined?
    // ___________________________________________________________________________________

    // Switch to Miner wallet context
    let miner_client = Client::new(
        &format!("{}/wallet/{}", RPC_URL, miner_wallet_name),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Generate one address from the Miner wallet with label "Mining Reward"
    let miner_address = miner_client.get_new_address(
        Some("Mining Reward"),
        Some(bitcoincore_rpc::json::AddressType::Bech32),
    )?;
    let mining_reward_address = miner_address
        .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)
        .map_err(|e| {
            bitcoincore_rpc::Error::ReturnedError(format!(
                "Failed to create miner address: {}",
                e.to_string()
            ))
        })?;

    println!("Miner address (Mining Reward): {}", mining_reward_address);

    // Mine new blocks to this address until you get positive wallet balance
    // In regtest, coinbase rewards mature after 100 blocks, so we need to mine 101 blocks
    // to have spendable balance from the first block
    let blocks_to_generate = 101;
    let block_hashes =
        miner_client.generate_to_address(blocks_to_generate, &mining_reward_address)?;
    println!("Generated {} blocks to miner address", blocks_to_generate);

    // Comment: Wallet balance for block rewards behaves this way because in Bitcoin,
    // coinbase transactions (block rewards) have a maturity period of 100 blocks in regtest mode.
    // This means that newly mined coins cannot be spent until 100 additional blocks are mined
    // on top of the block containing the coinbase transaction. This prevents issues with
    // blockchain reorganizations that could make spent coinbase outputs invalid.

    // Print the balance of the Miner wallet
    let miner_balance = miner_client.get_balance(None, None)?;
    println!("Miner wallet balance: {} BTC", miner_balance.to_btc());

    // ___________________________________________________________________________________
    // Load Trader wallet and generate a new address
    // ___________________________________________________________________________________

    // Send 20 BTC from Miner to Trader

    // Check transaction in mempool

    // Mine 1 block to confirm the transaction

    // Extract all required transaction details

    // Write the data to ../out.txt in the specified format given in readme.md

    Ok(())
}
