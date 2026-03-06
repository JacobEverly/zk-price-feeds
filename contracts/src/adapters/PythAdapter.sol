// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {ZkOracle} from "../ZkOracle.sol";

/// @notice Pyth PythStructs.Price return type.
struct PythPrice {
    int64 price;
    uint64 conf;
    int32 expo;
    uint publishTime;
}

/// @notice Minimal IPyth interface for price reading.
interface IPyth {
    function getPrice(bytes32 id) external view returns (PythPrice memory price);
    function getPriceUnsafe(bytes32 id) external view returns (PythPrice memory price);
    function getPriceNoOlderThan(bytes32 id, uint age) external view returns (PythPrice memory price);
    function getEmaPrice(bytes32 id) external view returns (PythPrice memory price);
    function getEmaPriceUnsafe(bytes32 id) external view returns (PythPrice memory price);
    function getEmaPriceNoOlderThan(bytes32 id, uint age) external view returns (PythPrice memory price);
    function getValidTimePeriod() external view returns (uint validTimePeriod);
}

/// @title ZkOraclePythAdapter
/// @notice Wraps a ZkOracle to expose the Pyth IPyth interface.
/// @dev Protocols using Pyth can point at this adapter with zero code changes.
///      The priceId is the keccak256 of the feed description for deterministic mapping.
///      EMA functions return the same spot price (ZK oracle computes median, not EMA).
contract ZkOraclePythAdapter is IPyth {
    ZkOracle public immutable oracle;

    /// @notice The price feed ID this adapter serves.
    bytes32 public immutable priceId;

    /// @notice Maximum age (seconds) before getPrice() reverts.
    uint public immutable validTimePeriod;

    error StalePrice();
    error PriceFeedNotFound();
    error NoRoundData();

    constructor(ZkOracle _oracle, uint _validTimePeriod) {
        oracle = _oracle;
        validTimePeriod = _validTimePeriod;
        // Deterministic priceId from the oracle's description
        priceId = keccak256(abi.encodePacked(_oracle.description()));
    }

    function getValidTimePeriod() external view override returns (uint) {
        return validTimePeriod;
    }

    /// @notice Returns the latest price, reverting if stale.
    function getPrice(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > validTimePeriod) revert StalePrice();
        return p;
    }

    /// @notice Returns the latest price with no staleness check.
    function getPriceUnsafe(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        return _latestPrice();
    }

    /// @notice Returns the latest price, reverting if older than `age` seconds.
    function getPriceNoOlderThan(bytes32 id, uint age) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > age) revert StalePrice();
        return p;
    }

    /// @notice EMA price (returns spot median -- ZK oracle doesn't compute EMA).
    function getEmaPrice(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > validTimePeriod) revert StalePrice();
        return p;
    }

    /// @notice EMA price unsafe (returns spot median).
    function getEmaPriceUnsafe(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        return _latestPrice();
    }

    /// @notice EMA price with age check (returns spot median).
    function getEmaPriceNoOlderThan(bytes32 id, uint age) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > age) revert StalePrice();
        return p;
    }

    // ─── Internal ──────────────────────────────────────────────────────

    function _checkId(bytes32 id) internal view {
        if (id != priceId) revert PriceFeedNotFound();
    }

    function _latestPrice() internal view returns (PythPrice memory) {
        uint80 roundId = oracle.currentRoundId();
        if (roundId == 0) revert NoRoundData();

        (, int256 answer,, uint256 updatedAt,) = oracle.latestRoundData();

        // Convert from oracle decimals to Pyth expo format.
        // Oracle stores price as fixed-point with PRICE_DECIMALS digits.
        // Pyth uses price * 10^expo, so expo = -int32(decimals).
        int32 expo = -int32(uint32(oracle.decimals()));

        return PythPrice({
            price: int64(answer),
            conf: 0, // ZK median has no confidence interval
            expo: expo,
            publishTime: updatedAt
        });
    }
}
