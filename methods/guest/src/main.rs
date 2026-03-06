#![no_main]

use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use risc0_steel::{Contract, EvmBlockHeader, ethereum::EthEvmInput};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

sol! {
    /// Uniswap V2 Pair interface -- getReserves returns the token reserves and last update timestamp.
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }
}

/// Journal committed by the guest, decoded on-chain by the verifier contract.
/// Contains the Steel commitment, the oracle parameters, and the computed price.
sol! {
    struct OracleJournal {
        /// Steel block commitment (version, block number, block hash, configID).
        bytes steelCommitment;
        /// Address of the Uniswap V2 pair contract.
        address pool;
        /// Median price as (reserve1 * 10^decimals) / reserve0.
        /// Represents the price of token0 denominated in token1.
        uint256 medianPrice;
        /// Number of decimals in the price (matches the `decimals` parameter).
        uint8 decimals;
        /// Number of blocks sampled.
        uint32 numSamples;
        /// Earliest block number in the sample window.
        uint64 startBlock;
        /// Latest block number in the sample window.
        uint64 endBlock;
    }
}

/// Guest input: oracle configuration passed from the host.
#[derive(serde::Deserialize)]
struct OracleConfig {
    /// Address of the Uniswap V2 pair contract.
    pool: Address,
    /// Number of decimals for the output price.
    decimals: u8,
}

fn main() {
    // Read the oracle configuration.
    let config: OracleConfig = env::read();

    // Read the number of block samples.
    let num_samples: u32 = env::read();

    let mut prices: Vec<U256> = Vec::with_capacity(num_samples as usize);
    let mut start_block: u64 = u64::MAX;
    let mut end_block: u64 = 0;
    let mut last_commitment: Option<Vec<u8>> = None;

    let scale = U256::from(10u64).pow(U256::from(config.decimals));

    // For each block sample, read the EVM input, verify state, and query reserves.
    for _ in 0..num_samples {
        let input: EthEvmInput = env::read();
        // Use Ethereum mainnet chain spec for commitment validation.
        let evm_env = input.into_env(&risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC);

        // Track block number from the header and commitment from the latest block.
        let block_number = evm_env.header().number();
        if block_number < start_block {
            start_block = block_number;
        }
        if block_number > end_block {
            end_block = block_number;
            last_commitment = Some(evm_env.commitment().abi_encode());
        }

        // Call getReserves() on the pool contract.
        let call = IUniswapV2Pair::getReservesCall {};
        let contract = Contract::new(config.pool, &evm_env);
        let returns = contract.call_builder(&call).call();

        let reserve0 = U256::from(returns.reserve0);
        let reserve1 = U256::from(returns.reserve1);

        // Price = (reserve1 * scale) / reserve0
        // This gives the price of token0 in terms of token1, scaled to `decimals` places.
        if !reserve0.is_zero() {
            let price = (reserve1 * scale) / reserve0;
            prices.push(price);
        }
    }

    assert!(!prices.is_empty(), "No valid price samples");

    // Compute median price.
    prices.sort();
    let median = if prices.len() % 2 == 0 {
        let mid = prices.len() / 2;
        (prices[mid - 1] + prices[mid]) / U256::from(2)
    } else {
        prices[prices.len() / 2]
    };

    let steel_commitment = last_commitment.expect("No blocks processed");

    // Encode and commit the journal.
    let journal = OracleJournal {
        steelCommitment: steel_commitment.into(),
        pool: config.pool,
        medianPrice: median,
        decimals: config.decimals,
        numSamples: num_samples,
        startBlock: start_block,
        endBlock: end_block,
    };

    env::commit_slice(&journal.abi_encode());
}
