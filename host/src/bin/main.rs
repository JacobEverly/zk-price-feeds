use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result};
use clap::Parser;
use risc0_steel::{
    Contract, EvmBlockHeader,
    ethereum::{ETH_MAINNET_CHAIN_SPEC, EthEvmEnv},
};
use risc0_zkvm::{ExecutorEnv, default_executor};
use tracing_subscriber::EnvFilter;
use url::Url;
use zk_oracle_methods::ZK_ORACLE_GUEST_ELF;

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
    #[arg(short, long, default_value = "1", env = "BLOCK_STRIDE")]
    stride: u64,

    /// Number of decimals for the output price.
    #[arg(short, long, default_value = "18", env = "PRICE_DECIMALS")]
    decimals: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    println!("ZK Price Oracle");
    println!("  Pool:       {:#}", args.pool);
    println!("  Samples:    {}", args.num_blocks);
    println!("  Stride:     {}", args.stride);
    println!("  Decimals:   {}", args.decimals);
    println!();

    // Determine the latest block to anchor our sampling window.
    let latest_env = EthEvmEnv::builder()
        .rpc(args.rpc_url.clone())
        .chain_spec(&ETH_MAINNET_CHAIN_SPEC)
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
            .rpc(args.rpc_url.clone())
            .block_number(block_num)
            .chain_spec(&ETH_MAINNET_CHAIN_SPEC)
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

    let mut builder = ExecutorEnv::builder();
    builder.write(&config)?;
    builder.write(&args.num_blocks)?;
    for input in inputs {
        builder.write(&input)?;
    }
    let executor_env = builder.build().context("failed to build executor env")?;

    println!("\nRunning the guest program...");
    let exec = default_executor();
    let session_info = exec
        .execute(executor_env, ZK_ORACLE_GUEST_ELF)
        .context("failed to run executor")?;

    // Decode the journal output.
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

    Ok(())
}
