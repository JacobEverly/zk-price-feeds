// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {ZkOracle} from "../ZkOracle.sol";

/// @notice Minimal IChronicle interface for price reading.
interface IChronicle {
    function wat() external view returns (bytes32);
    function read() external view returns (uint);
    function readWithAge() external view returns (uint value, uint age);
    function tryRead() external view returns (bool isValid, uint value);
    function tryReadWithAge() external view returns (bool isValid, uint value, uint age);
}

/// @notice Minimal IToll interface for access control.
interface IToll {
    function kiss(address who) external;
    function diss(address who) external;
    function tolled(address who) external view returns (bool);
}

/// @title ZkOracleChronicleAdapter
/// @notice Wraps a ZkOracle to expose the Chronicle IChronicle + IToll interface.
/// @dev Protocols using Chronicle can point at this adapter with zero code changes.
///      Includes a toll (whitelist) system matching Chronicle's access pattern.
///      Chronicle uses 18-decimal fixed-point; this adapter scales from oracle decimals.
contract ZkOracleChronicleAdapter is IChronicle, IToll {
    ZkOracle public immutable oracle;

    /// @notice Feed identifier (e.g. keccak256("ETH/USD")).
    bytes32 public immutable feedWat;

    /// @notice Owner who can manage the toll whitelist.
    address public owner;

    /// @notice Toll whitelist -- addresses allowed to read.
    mapping(address => bool) internal _tolled;

    /// @notice Chronicle uses 18-decimal fixed-point.
    uint8 internal constant CHRONICLE_DECIMALS = 18;

    error NotTolled(address caller);
    error NotOwner();
    error NoRoundData();

    modifier onlyTolled() {
        if (!_tolled[msg.sender]) revert NotTolled(msg.sender);
        _;
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(ZkOracle _oracle, bytes32 _wat) {
        oracle = _oracle;
        feedWat = _wat;
        owner = msg.sender;
        // Deployer is tolled by default.
        _tolled[msg.sender] = true;
    }

    // ─── IChronicle ────────────────────────────────────────────────────

    /// @notice Returns the feed identifier.
    function wat() external view override returns (bytes32) {
        return feedWat;
    }

    /// @notice Returns the current price in 18-decimal fixed-point. Reverts if no data.
    function read() external view override onlyTolled returns (uint) {
        return _getPrice();
    }

    /// @notice Returns current price and age (timestamp). Reverts if no data.
    function readWithAge() external view override onlyTolled returns (uint value, uint age) {
        (value, age) = _getPriceWithAge();
    }

    /// @notice Returns price without reverting on absence.
    function tryRead() external view override onlyTolled returns (bool isValid, uint value) {
        if (oracle.currentRoundId() == 0) return (false, 0);
        return (true, _getPrice());
    }

    /// @notice Returns price and age without reverting on absence.
    function tryReadWithAge() external view override onlyTolled returns (bool isValid, uint value, uint age) {
        if (oracle.currentRoundId() == 0) return (false, 0, 0);
        (value, age) = _getPriceWithAge();
        isValid = true;
    }

    // ─── IToll ─────────────────────────────────────────────────────────

    /// @notice Grant read access to `who`.
    function kiss(address who) external override onlyOwner {
        _tolled[who] = true;
    }

    /// @notice Revoke read access from `who`.
    function diss(address who) external override onlyOwner {
        _tolled[who] = false;
    }

    /// @notice Check if `who` has read access.
    function tolled(address who) external view override returns (bool) {
        return _tolled[who];
    }

    // ─── Internal ──────────────────────────────────────────────────────

    function _getPrice() internal view returns (uint) {
        (uint value,) = _getPriceWithAge();
        return value;
    }

    function _getPriceWithAge() internal view returns (uint value, uint age) {
        if (oracle.currentRoundId() == 0) revert NoRoundData();

        (, int256 answer,, uint256 updatedAt,) = oracle.latestRoundData();

        // Scale from oracle decimals to Chronicle's 18-decimal format.
        uint8 oracleDecimals = oracle.decimals();
        if (oracleDecimals < CHRONICLE_DECIMALS) {
            value = uint256(answer) * 10 ** (CHRONICLE_DECIMALS - oracleDecimals);
        } else if (oracleDecimals > CHRONICLE_DECIMALS) {
            value = uint256(answer) / 10 ** (oracleDecimals - CHRONICLE_DECIMALS);
        } else {
            value = uint256(answer);
        }

        age = updatedAt;
    }
}
