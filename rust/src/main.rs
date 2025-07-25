#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::{Amount, SignedAmount};
use bitcoincore_rpc::{Auth, Client, RawTx, RpcApi};
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
    println!("Blockchain Info: {blockchain_info:?}");

    // ___________________________________________________________________________________
    // Create/Load the wallets, named 'Miner' and 'Trader'. Have logic to optionally
    // create/load them if they do not exist or not loaded already.
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
        &format!("{RPC_URL}/wallet/{miner_wallet_name}"),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Generate one address from the Miner wallet with label "Mining Reward"
    let miner_address = miner_client.get_new_address(
        Some("Mining Reward"),
        Some(bitcoincore_rpc::json::AddressType::Bech32),
    )?;
    let mining_reward_address = miner_address
        .clone()
        .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)
        .map_err(|e| {
            bitcoincore_rpc::Error::ReturnedError(format!("Failed to create miner address: {e}"))
        })?;

    println!("Miner address (Mining Reward): {mining_reward_address}");

    // Mine new blocks to this address until you get positive wallet balance
    // In regtest, coinbase rewards mature after 100 blocks, so we need to mine 101 blocks
    // to have spendable balance from the first block
    let blocks_to_generate = 101;
    let block_hashes =
        miner_client.generate_to_address(blocks_to_generate, &mining_reward_address)?;
    println!("Generated {blocks_to_generate} blocks to miner address");

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

    // Switch to Trader wallet context
    let trader_client = Client::new(
        &format!("{RPC_URL}/wallet/{trader_wallet_name}"),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Create a receiving address labeled "Received" from Trader wallet
    let trader_address = trader_client.get_new_address(
        Some("Received"),
        Some(bitcoincore_rpc::json::AddressType::Bech32),
    )?;
    let trader_receive_address = trader_address
        .clone()
        .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)
        .map_err(|e| {
            bitcoincore_rpc::Error::ReturnedError(format!("Failed to create trader address: {e}"))
        })?;
    println!("Trader address (Received): {trader_receive_address}");

    // ___________________________________________________________________________________
    // Send 20 BTC from Miner to Trader
    // ___________________________________________________________________________________

    // Send a transaction paying 20 BTC from Miner wallet to Trader's wallet
    let send_amount = Amount::from_btc(20.0).unwrap();
    let txid = miner_client.send_to_address(
        &trader_receive_address,
        send_amount,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    println!("Transaction ID: {txid}");

    // ___________________________________________________________________________________
    // Check transaction in mempool
    // ___________________________________________________________________________________

    // Fetch the unconfirmed transaction from the node's mempool
    let mempool_entry = rpc.get_mempool_entry(&txid)?;
    println!("Mempool entry: {mempool_entry:?}");

    // ____________________________________________________________________________________
    // Mine 1 block to confirm the transaction
    // ____________________________________________________________________________________

    // Confirm the transaction by mining 1 block
    let confirmation_block = rpc.generate_to_address(1, &mining_reward_address)?;
    let block_hash = confirmation_block[0];
    println!("Transaction confirmed in block: {block_hash}");

    // ____________________________________________________________________________________
    // Extract all required transaction details
    // ____________________________________________________________________________________

    // Get the raw transaction first
    let miner_tx = miner_client.get_raw_transaction(&txid, None)?;

    // Miner's Change Address
    let miner_raw_tx = miner_client.decode_raw_transaction(&miner_tx, Some(true))?;

    println!("Transaction outputs:");
    for (i, vout) in miner_raw_tx.vout.iter().enumerate() {
        println!("Output {}: {:?}", i, vout.script_pub_key.address);
    }
    println!("Trader address: {trader_receive_address}");

    // Handle the case where there might be no change output
    let miner_vout_option = miner_raw_tx.vout.iter().find(|v| {
        if let Some(addr) = &v.script_pub_key.address {
            addr != &trader_receive_address
        } else {
            false
        }
    });

    // Get the raw transaction
    let raw_tx = miner_client.get_raw_transaction(&txid, None)?;
    let decoded_tx = miner_client.decode_raw_transaction(&raw_tx, Some(true))?;

    // Find change output by comparing against trader address
    let trader_addr_str = trader_receive_address.to_string();

    let mut change_address = mining_reward_address.clone(); // fallback
    let mut change_amount = 0.0;

    // Look through all outputs to find the change
    for vout in &decoded_tx.vout {
        if let Some(addr) = &vout.script_pub_key.address {
            // Convert address to string for comparison
            let output_addr = addr
                .clone()
                .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)
                .map_err(|e| {
                    bitcoincore_rpc::Error::ReturnedError(format!(
                        "Failed to process output address: {e}"
                    ))
                })?;

            let output_addr_str = output_addr.to_string();
            println!(
                "Checking output: {} BTC to {output_addr_str}",
                vout.value.to_btc()
            );

            // If this output is NOT going to the trader, it's the change
            if output_addr_str != trader_addr_str {
                change_address = output_addr;
                change_amount = vout.value.to_btc();
                println!("Found change output: {change_amount} BTC to {change_address}");
                break;
            }
        }
    }

    // If no change was found, there might be an issue with the transaction
    if change_amount == 0.0 {
        println!("Warning: No change output found. This might indicate:");
        println!("1. The input amount exactly equals output + fees");
        println!("2. There's an issue with address comparison");
        println!("3. The transaction structure is different than expected");

        // Let's examine all outputs more carefully
        println!("All transaction outputs:");
        for (i, vout) in decoded_tx.vout.iter().enumerate() {
            println!("  Output {}: {} BTC", i, vout.value.to_btc());
            if let Some(addr) = &vout.script_pub_key.address {
                let addr_str = addr
                    .clone()
                    .require_network(bitcoincore_rpc::bitcoin::Network::Regtest)
                    .map_err(|e| {
                        bitcoincore_rpc::Error::ReturnedError(format!("Address error: {e}"))
                    })?
                    .to_string();
                println!("    Address: {addr_str}");
                println!("    Is trader address? {}", addr_str == trader_addr_str);
            }
        }
    }

    // Get transaction details using the miner client (since it sent the transaction)
    let tx_details = miner_client.get_transaction(&txid, Some(true))?;
    let raw_tx_info = rpc.get_raw_transaction_info(&txid, None)?;
    let block_info = rpc.get_block(&block_hash)?;
    let block_height = rpc.get_block_count()?;

    // Extract input information
    let input_amount = tx_details
        .details
        .iter()
        .find(|d| d.category == bitcoincore_rpc::json::GetTransactionResultDetailCategory::Send)
        .map(|d| d.amount.to_btc().abs())
        .unwrap_or(0.0);

    let output_amount = 20.0; // We sent 20 BTC
    let fee = tx_details.fee.unwrap_or(SignedAmount::ZERO).to_btc().abs();

    // Convert trader address to string for comparison
    let trader_addr_str = trader_receive_address.to_string();

    // Extract output info
    let tx_details = miner_client.get_transaction(&txid, Some(true))?;
    let tx = tx_details.transaction().unwrap(); // Fully decoded transaction
    // let fee = tx_details
    //     .fee
    //     .unwrap_or(SignedAmount::from_btc(0.0).unwrap());
    let fee = tx_details.fee.unwrap_or(SignedAmount::ZERO).to_btc().abs();

    let outputs = &tx.output;
    let mut trader_output = None;
    let mut change_output = None;

    for out in outputs {
        let out_address = bitcoincore_rpc::bitcoin::Address::from_script(&out.script_pubkey, bitcoincore_rpc::bitcoin::Network::Regtest).unwrap();
        if out_address == trader_address {
            trader_output = Some((out_address, out.value));
        } else {
            change_output = Some((out_address, out.value));
        }
    }

    println!("Looking for change address (trader address: {trader_addr_str})");
    println!("Change address: {change_address}");

    // ____________________________________________________________________________________
    // Write the data to ../out.txt in the specified format given in readme.md
    // ____________________________________________________________________________________

    // Format the data to the expected format
    let output_content = format!(
        "{txid}\n{mining_reward_address}\n{input_amount}\n{trader_receive_address}\n{output_amount}\n{change_address}\n{change_amount}\n{fee}\n{block_height}\n{block_hash}"
        
    );
    println!("\nOutput content:\n{output_content}");

    let mut file = File::create("../out.txt")?;
    file.write_all(output_content.as_bytes())?;
    println!("\nTransaction details written to out.txt");

    Ok(())
}
