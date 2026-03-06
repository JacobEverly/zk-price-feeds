#!/usr/bin/env python3
"""Query all Uniswap V2 pairs and compute ZK Oracle benchmark data."""

import subprocess
import json
import sys
import time

RPC = "https://ethereum-rpc.publicnode.com"

# (display_name, pool_address, token0_symbol, token0_decimals, token1_symbol, token1_decimals, price_display)
# price_display: "t0_in_t1" means price of token0 in token1, "t1_in_t0" means invert
PAIRS = [
    # Major pairs
    ("ETH/USDC",  "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", "USDC", 6, "WETH", 18, "t1_in_t0"),
    ("ETH/USDT",  "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", "WETH", 18, "USDT", 6, "t0_in_t1"),
    ("BTC/USDC",  "0x004375Dff511095CC5A197A54140a24eFEF3A416", "WBTC", 8, "USDC", 6, "t0_in_t1"),
    ("ETH/DAI",   "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", "DAI", 18, "WETH", 18, "t1_in_t0"),
    ("BTC/ETH",   "0xBb2b8038a1640196FbE3e38816F3e67Cba72D940", "WBTC", 8, "WETH", 18, "t0_in_t1"),

    # DeFi blue chips
    ("UNI/ETH",   "0xd3d2E2692501A5c9Ca623199D38826e513033a17", "UNI", 18, "WETH", 18, "t0_in_t1"),
    ("LINK/ETH",  "0xa2107FA5B38d9bbd2C461D6EDf11B11A50F6b974", "LINK", 18, "WETH", 18, "t0_in_t1"),
    ("AAVE/ETH",  "0xDFC14d2Af169B0D36C4EFF567Ada9b2E0CAE044f", "AAVE", 18, "WETH", 18, "t0_in_t1"),
    ("MKR/ETH",   "0xC2aDdA861F89bBB333c90c492cB837741916A225", "MKR", 18, "WETH", 18, "t0_in_t1"),
    ("CRV/ETH",   "0x3dA1313aE46132A397D90d95B1424A9A7e3e0fCE", "CRV", 18, "WETH", 18, "t0_in_t1"),
    ("COMP/ETH",  "0xCFfDdeD873554F362Ac02f8Fb1f02E5ada10516f", "COMP", 18, "WETH", 18, "t0_in_t1"),
    ("SNX/ETH",   "0x43AE24960e5534731Fc831386c07755A2dc33D47", "SNX", 18, "WETH", 18, "t0_in_t1"),

    # Stablecoins
    ("USDC/USDT", "0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f", "USDC", 6, "USDT", 6, "t0_in_t1"),
    ("DAI/USDC",  "0xAE461cA67B15dc8dc81CE7615e0320dA1A9aB8D5", "DAI", 18, "USDC", 6, "t0_in_t1"),

    # Infra
    ("LDO/ETH",   "0xC558F600B34A5f69dD2F0D06Cb8A88d829B7420a", "LDO", 18, "WETH", 18, "t0_in_t1"),
    ("RPL/ETH",   "0x70EA56e46266f0137BAc6B75710e3546f47C855D", "RPL", 18, "WETH", 18, "t0_in_t1"),

    # Meme / long tail
    ("SHIB/ETH",  "0x811beEd0119b4AfCE20D2583EB608C6F7AF1954f", "SHIB", 18, "WETH", 18, "t0_in_t1"),
    ("PEPE/ETH",  "0xA43fe16908251ee70EF3045e6c8003a0BfD3eCAd", "PEPE", 18, "WETH", 18, "t0_in_t1"),
]

# ZK Oracle constants
CYCLES_PER_BLOCK = 3.67e6  # from benchmarks
BLOCKS = 20
TOTAL_CYCLES = CYCLES_PER_BLOCK * BLOCKS
COST_PER_BILLION = 0.05  # $0.05 per billion cycles
PROVING_COST = (TOTAL_CYCLES / 1e9) * COST_PER_BILLION
PROVING_LATENCY_S = 15
ONCHAIN_GAS = 325_000
ETH_PRICE_USD = None  # filled from first query


def cast_call(address, sig, rpc=RPC):
    """Call a contract using cast."""
    try:
        result = subprocess.run(
            ["cast", "call", address, sig, "--rpc-url", rpc],
            capture_output=True, text=True, timeout=30
        )
        if result.returncode != 0:
            return None
        return result.stdout.strip()
    except Exception:
        return None


def get_reserves(pool):
    """Get reserves from a Uniswap V2 pair."""
    raw = cast_call(pool, "getReserves()(uint112,uint112,uint32)")
    if not raw:
        return None, None, None
    lines = raw.strip().split("\n")
    if len(lines) < 3:
        return None, None, None

    def parse_val(s):
        # Handle cast output like "9336084485254 [9.336e12]"
        return int(s.strip().split()[0])

    r0 = parse_val(lines[0])
    r1 = parse_val(lines[1])
    ts = parse_val(lines[2])
    return r0, r1, ts


def compute_price(r0, r1, t0_dec, t1_dec, direction):
    """Compute human-readable price."""
    real0 = r0 / (10 ** t0_dec)
    real1 = r1 / (10 ** t1_dec)

    if direction == "t0_in_t1":
        # Price of token0 in terms of token1
        if real0 == 0:
            return 0
        return real1 / real0
    else:
        # Price of token1 in terms of token0
        if real1 == 0:
            return 0
        return real0 / real1


def get_block_number():
    raw = cast_call("0x0000000000000000000000000000000000000000",
                     "getReserves()(uint112,uint112,uint32)")  # dummy
    # Use eth_blockNumber instead
    result = subprocess.run(
        ["cast", "block-number", "--rpc-url", RPC],
        capture_output=True, text=True, timeout=15
    )
    if result.returncode == 0:
        return int(result.stdout.strip())
    return 0


def format_price(price, name):
    """Format price for display."""
    if price >= 10000:
        return f"${price:,.0f}"
    elif price >= 1:
        return f"${price:,.2f}"
    elif price >= 0.001:
        return f"${price:.4f}"
    elif price >= 0.000001:
        return f"${price:.8f}"
    else:
        return f"${price:.12f}"


def format_liquidity(r0, r1, t0_dec, t1_dec, eth_price):
    """Estimate pool TVL in USD."""
    real0 = r0 / (10 ** t0_dec)
    real1 = r1 / (10 ** t1_dec)
    # Very rough: multiply both sides by their USD value
    # This is a simplification
    return real0, real1


def main():
    global ETH_PRICE_USD

    block = get_block_number()
    print(f"Block: {block}")
    print(f"Querying {len(PAIRS)} Uniswap V2 pairs...\n")

    results = []

    for name, pool, t0_sym, t0_dec, t1_sym, t1_dec, direction in PAIRS:
        time.sleep(0.3)  # rate limit
        r0, r1, ts = get_reserves(pool)
        if r0 is None:
            print(f"  SKIP {name}: failed to query {pool}")
            continue

        price = compute_price(r0, r1, t0_dec, t1_dec, direction)

        # Get ETH price from ETH/USDC pair
        if name == "ETH/USDC" and ETH_PRICE_USD is None:
            ETH_PRICE_USD = price

        results.append({
            "name": name,
            "pool": pool,
            "r0": r0,
            "r1": r1,
            "t0_sym": t0_sym,
            "t0_dec": t0_dec,
            "t1_sym": t1_sym,
            "t1_dec": t1_dec,
            "price": price,
            "direction": direction,
        })

        print(f"  {name:<12} price={format_price(price, name):<18} r0={r0:<25} r1={r1}")

    if ETH_PRICE_USD is None:
        ETH_PRICE_USD = 2082  # fallback

    # Now compute USD prices for ETH-denominated pairs
    print(f"\nETH price: ${ETH_PRICE_USD:,.2f}")
    print(f"Proving cost (20 blocks): ${PROVING_COST:.4f}")
    print(f"Proving latency: {PROVING_LATENCY_S}s")
    print()

    # Gas costs at various gas prices
    gas_1 = ONCHAIN_GAS * 1e-9 * ETH_PRICE_USD
    gas_5 = ONCHAIN_GAS * 5e-9 * ETH_PRICE_USD
    gas_l2 = ONCHAIN_GAS * 0.01e-9 * ETH_PRICE_USD  # L2 ~0.01 Gwei effective

    # Output the full table
    print("=" * 140)
    print(f"{'Pair':<12} {'Price (USD)':<18} {'Pool Liquidity':<25} {'Cycles (20blk)':<15} {'Proving $':<12} {'L1 Gas (5gw)':<14} {'L1 Total':<12} {'L2 Total':<12} {'Has CL Feed'}")
    print("=" * 140)

    # Chainlink supported feeds (approximate list)
    chainlink_feeds = {"ETH/USDC", "ETH/USDT", "BTC/USDC", "ETH/DAI", "BTC/ETH",
                       "UNI/ETH", "LINK/ETH", "AAVE/ETH", "MKR/ETH", "COMP/ETH",
                       "SNX/ETH", "USDC/USDT", "DAI/USDC", "CRV/ETH", "LDO/ETH"}

    for r in results:
        name = r["name"]
        price = r["price"]

        # Convert to USD if ETH-denominated
        if r["t1_sym"] == "WETH" and r["direction"] == "t0_in_t1":
            usd_price = price * ETH_PRICE_USD
        elif r["t0_sym"] == "WETH" and r["direction"] == "t1_in_t0":
            usd_price = price  # already USD
        elif r["t1_sym"] in ("USDC", "USDT") and r["direction"] == "t0_in_t1":
            usd_price = price
        elif r["t0_sym"] in ("USDC", "USDT", "DAI") and r["direction"] == "t1_in_t0":
            usd_price = price
        elif r["t0_sym"] == "DAI" and r["direction"] == "t1_in_t0":
            usd_price = price
        elif name == "ETH/DAI":
            usd_price = price  # DAI ~ $1
        elif name == "DAI/USDC":
            usd_price = price  # already ~$1
        elif name == "USDC/USDT":
            usd_price = price  # already ~$1
        else:
            usd_price = price

        # Pool liquidity estimate
        real0 = r["r0"] / (10 ** r["t0_dec"])
        real1 = r["r1"] / (10 ** r["t1_dec"])

        # Estimate TVL
        if r["t0_sym"] in ("USDC", "USDT", "DAI"):
            tvl = real0 * 2  # double the stablecoin side
        elif r["t1_sym"] in ("USDC", "USDT", "DAI"):
            tvl = real1 * 2
        elif r["t0_sym"] == "WETH":
            tvl = real0 * ETH_PRICE_USD * 2
        elif r["t1_sym"] == "WETH":
            tvl = real1 * ETH_PRICE_USD * 2
        else:
            tvl = 0

        tvl_str = f"${tvl:,.0f}" if tvl > 0 else "N/A"
        has_cl = "Yes" if name in chainlink_feeds else "No"

        l1_total = PROVING_COST + gas_5
        l2_total = PROVING_COST + gas_l2

        print(f"{name:<12} {format_price(usd_price, name):<18} {tvl_str:<25} {TOTAL_CYCLES/1e6:.1f}M{'':<9} ${PROVING_COST:<10.4f} ${gas_5:<12.2f} ${l1_total:<10.2f} ${l2_total:<10.4f} {has_cl}")

    # Summary stats
    print()
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Pairs benchmarked:     {len(results)}")
    print(f"Block:                 {block}")
    print(f"ETH price:             ${ETH_PRICE_USD:,.2f}")
    print(f"Cycles per update:     {TOTAL_CYCLES/1e6:.1f}M ({BLOCKS} blocks)")
    print(f"Proving cost:          ${PROVING_COST:.4f}")
    print(f"Proving latency:       {PROVING_LATENCY_S}s (Boundless marketplace)")
    print(f"On-chain gas:          {ONCHAIN_GAS:,} gas")
    print(f"L1 total (5 Gwei):     ${PROVING_COST + gas_5:.2f}")
    print(f"L2 total (~0.01 Gwei): ${PROVING_COST + gas_l2:.4f}")
    print(f"Max update frequency:  Every {PROVING_LATENCY_S}s (limited by proving latency)")
    print()
    print("Update cadence cost estimates (L1 at 5 Gwei):")
    for interval, label in [(60, "Every 1 min"), (300, "Every 5 min"), (3600, "Every 1 hour"), (86400, "Every 24 hours")]:
        updates_per_day = 86400 / interval
        daily_cost = updates_per_day * (PROVING_COST + gas_5)
        print(f"  {label:<20} {updates_per_day:>6.0f} updates/day  ${daily_cost:>10,.2f}/day  ${daily_cost*30:>12,.2f}/month")

    print()
    print("Update cadence cost estimates (L2 at ~0.01 Gwei):")
    for interval, label in [(60, "Every 1 min"), (300, "Every 5 min"), (3600, "Every 1 hour"), (86400, "Every 24 hours")]:
        updates_per_day = 86400 / interval
        daily_cost = updates_per_day * (PROVING_COST + gas_l2)
        print(f"  {label:<20} {updates_per_day:>6.0f} updates/day  ${daily_cost:>10.2f}/day  ${daily_cost*30:>12.2f}/month")

if __name__ == "__main__":
    main()
