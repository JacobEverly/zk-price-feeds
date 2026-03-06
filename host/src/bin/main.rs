use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result, ensure};
use clap::Parser;
use risc0_steel::{
    Contract, EvmBlockHeader,
    alloy::{
        network::EthereumWallet,
        providers::{Provider, ProviderBuilder},
        signers::local::PrivateKeySigner,
    },
    ethereum::{EthChainSpec, EthEvmEnv},
};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{Digest, ExecutorEnv, Prover, ProverOpts, default_executor, default_prover};
use tracing_subscriber::EnvFilter;
use url::Url;
use zk_oracle_methods::{ZK_ORACLE_GUEST_ELF, ZK_ORACLE_GUEST_ID};

sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }
}

// Journal structure matching the guest program's output.
sol! {
    struct OracleJournal {
        bytes steelCommitment;
        address pool;
        uint256 medianPrice;
        uint8 decimals;
        uint32 numSamples;
        uint64 startBlock;
        uint64 endBlock;
    }
}

// On-chain ZkOracle contract interface for submitting proofs.
sol! {
    #[sol(rpc)]
    interface IZkOracle {
        function updatePrice(bytes calldata journalData, bytes calldata seal) external;
        function imageId() external view returns (bytes32);
        function currentRoundId() external view returns (uint80);
        function latestRoundData() external view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
    }
}

/// Guest input: oracle configuration passed to the guest.
#[derive(serde::Serialize)]
struct OracleConfig {
    pool: Address,
    decimals: u8,
}

/// ZK Price Oracle -- proves DEX prices across multiple blocks using Steel.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the Ethereum RPC endpoint.
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,

    /// Address of the Uniswap V2 pair contract.
    #[arg(short, long, env = "POOL_ADDRESS")]
    pool: Address,

    /// Number of blocks to sample for the median price.
    #[arg(short, long, default_value = "10", env = "NUM_BLOCKS")]
    num_blocks: u32,

    /// Block stride -- sample every Nth block (1 = consecutive blocks).
    #[arg(short = 's', long, default_value = "1", env = "BLOCK_STRIDE")]
    stride: u64,

    /// Number of decimals for the output price.
    #[arg(short, long, default_value = "18", env = "PRICE_DECIMALS")]
    decimals: u8,

    /// Address of the deployed ZkOracle contract (for on-chain submission).
    /// If omitted, runs in execute-only mode (no proof, no on-chain tx).
    #[arg(long, env = "ORACLE_ADDRESS")]
    oracle_address: Option<Address>,

    /// Private key for submitting the on-chain transaction.
    /// Required when --oracle-address is set.
    #[arg(long, env = "ETH_WALLET_PRIVATE_KEY")]
    private_key: Option<PrivateKeySigner>,

    /// Run in execute-only mode (no ZK proof generation).
    /// Useful for testing the guest program locally.
    #[arg(long, default_value = "false")]
    execute_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let proving = !args.execute_only;
    let submitting = args.oracle_address.is_some();

    if submitting {
        ensure!(
            args.private_key.is_some(),
            "--private-key is required when --oracle-address is set"
        );
    }

    println!("ZK Price Oracle");
    println!("  Pool:       {:#}", args.pool);
    println!("  Samples:    {}", args.num_blocks);
    println!("  Stride:     {}", args.stride);
    println!("  Decimals:   {}", args.decimals);
    println!("  Mode:       {}", if submitting { "prove + submit" } else if proving { "prove only" } else { "execute only" });
    println!();

    // Build a provider (with or without wallet depending on mode).
    let provider = if let Some(ref pk) = args.private_key {
        let wallet = EthereumWallet::from(pk.clone());
        ProviderBuilder::new()
            .wallet(wallet)
            .connect_http(args.rpc_url.clone())
    } else {
        ProviderBuilder::new().connect_http(args.rpc_url.clone())
    };

    // Detect chain spec from chain ID.
    let chain_id = provider.get_chain_id().await?;
    let chain_spec = EthChainSpec::from_chain_id(chain_id)
        .with_context(|| format!("Unsupported chain ID: {chain_id}"))?;

    // Determine the latest block to anchor our sampling window.
    let latest_env = EthEvmEnv::builder()
        .provider(provider.clone())
        .chain_spec(chain_spec.clone())
        .build()
        .await?;
    let latest_block = latest_env.header().number();
    println!("Latest block: {latest_block}");

    // Calculate which blocks to sample (working backwards from latest).
    let block_numbers: Vec<u64> = (0..args.num_blocks as u64)
        .map(|i| latest_block - (i * args.stride))
        .collect();

    println!(
        "Sampling blocks {} to {} (stride {})",
        block_numbers.last().unwrap(),
        block_numbers[0],
        args.stride
    );

    // Build Steel environments and preflight each block.
    let mut inputs = Vec::with_capacity(args.num_blocks as usize);
    let call = IUniswapV2Pair::getReservesCall {};

    for &block_num in &block_numbers {
        let mut env = EthEvmEnv::builder()
            .provider(provider.clone())
            .block_number(block_num)
            .chain_spec(chain_spec.clone())
            .build()
            .await
            .with_context(|| format!("failed to build env for block {block_num}"))?;

        // Preflight the getReserves call to prepare the storage proofs.
        let mut contract = Contract::preflight(args.pool, &mut env);
        let returns = contract.call_builder(&call).call().await?;

        let reserve0 = U256::from(returns.reserve0);
        let reserve1 = U256::from(returns.reserve1);
        let scale = U256::from(10u64).pow(U256::from(args.decimals));
        let price = if !reserve0.is_zero() {
            (reserve1 * scale) / reserve0
        } else {
            U256::ZERO
        };
        println!(
            "  Block {block_num}: reserve0={reserve0}, reserve1={reserve1}, price={price}"
        );

        let input = env.into_input().await?;
        inputs.push(input);
    }

    // Build the guest executor environment.
    let config = OracleConfig {
        pool: args.pool,
        decimals: args.decimals,
    };

    let num_blocks = args.num_blocks;
    let mut builder = ExecutorEnv::builder();
    builder.write(&config)?;
    builder.write(&num_blocks)?;
    for input in inputs {
        builder.write(&input)?;
    }
    let executor_env = builder.build().context("failed to build executor env")?;

    if args.execute_only {
        // Execute-only mode: run the guest without proof generation.
        println!("\nRunning the guest program (execute only, no proof)...");
        let exec = default_executor();
        let session_info = exec
            .execute(executor_env, ZK_ORACLE_GUEST_ELF)
            .context("failed to run executor")?;

        let journal =
            OracleJournal::abi_decode(session_info.journal.as_ref()).context("failed to decode journal")?;

        println!("\nOracle Result:");
        println!("  Pool:         {:#}", journal.pool);
        println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.decimals);
        println!("  Samples:      {}", journal.numSamples);
        println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);
        println!("  Segments:     {}", session_info.segments.len());
        println!(
            "  Total Cycles: {}",
            session_info.segments.iter().map(|s| 1 << s.po2).sum::<u64>()
        );
    } else {
        // Proving mode: generate a Groth16 ZK proof.
        println!("\nGenerating ZK proof (Groth16)...");
        let prove_info = tokio::task::spawn_blocking(move || {
            default_prover().prove_with_opts(
                executor_env,
                ZK_ORACLE_GUEST_ELF,
                &ProverOpts::groth16(),
            )
        })
        .await?
        .context("failed to generate proof")?;

        let receipt = prove_info.receipt;
        let journal_bytes = receipt.journal.bytes.clone();

        let journal =
            OracleJournal::abi_decode(&journal_bytes).context("failed to decode journal")?;

        println!("\nOracle Result:");
        println!("  Pool:         {:#}", journal.pool);
        println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.decimals);
        println!("  Samples:      {}", journal.numSamples);
        println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);

        // Encode the seal for on-chain verification.
        let seal = encode_seal(&receipt).context("failed to encode seal")?;
        println!("  Seal size:    {} bytes", seal.len());

        if let Some(oracle_address) = args.oracle_address {
            // Submit to the on-chain ZkOracle contract.
            println!("\nSubmitting proof on-chain...");

            let contract = IZkOracle::new(oracle_address, &provider);

            // Verify image ID matches.
            let on_chain_image_id = contract.imageId().call().await?;
            ensure!(
                on_chain_image_id.0 == <[u8; 32]>::from(Digest::from(ZK_ORACLE_GUEST_ID)),
                "Image ID mismatch between guest program and on-chain contract. Redeploy the contract with the correct image ID."
            );

            // Call updatePrice(journalData, seal).
            let call_builder = contract.updatePrice(journal_bytes.into(), seal.into());
            let pending_tx = call_builder.send().await?;
            let tx_hash = *pending_tx.tx_hash();
            println!("  Tx sent: {tx_hash:#}");

            let tx_receipt = pending_tx
                .get_receipt()
                .await
                .with_context(|| format!("transaction did not confirm: {tx_hash}"))?;
            ensure!(tx_receipt.status(), "transaction reverted: {tx_hash}");

            // Read back the updated price.
            let round_id = contract.currentRoundId().call().await?;
            let latest = contract.latestRoundData().call().await?;

            println!("\n✅ Price updated on-chain!");
            println!("  Round:   {}", round_id.0);
            println!("  Price:   {}", latest.answer);
            println!("  Updated: {}", latest.updatedAt);
            println!("  Tx:      {tx_hash:#}");
        } else {
            println!("\nProof generated successfully. Use --oracle-address to submit on-chain.");
        }
    }

    Ok(())
}
