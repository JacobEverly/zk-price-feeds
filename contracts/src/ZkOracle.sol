// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {Steel} from "risc0-steel/Steel.sol";

/// @title IAggregatorV3Interface
/// @notice Chainlink-compatible price feed interface for drop-in replacement.
interface IAggregatorV3Interface {
    function decimals() external view returns (uint8);
    function description() external view returns (string memory);
    function version() external pure returns (uint256);

    function getRoundData(uint80 _roundId)
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);

    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
}

/// @title ZkOracle
/// @notice A ZK-verified price oracle implementing the Chainlink AggregatorV3Interface.
/// @dev Prices are proven from on-chain DEX reserves using Steel (RISC Zero zkVM).
///      Anyone can submit a proof to update the price feed.
contract ZkOracle is IAggregatorV3Interface {
    /// @notice RISC Zero verifier contract.
    IRiscZeroVerifier public immutable VERIFIER;

    /// @notice Image ID of the ZK oracle guest program.
    bytes32 public immutable IMAGE_ID;

    /// @notice Address of the DEX pool being tracked.
    address public immutable POOL;

    /// @notice Price decimals.
    uint8 internal immutable PRICE_DECIMALS;

    /// @notice Human-readable description (e.g. "ETH / USDC").
    string internal feedDescription;

    /// @notice Stored round data.
    struct Round {
        int256 answer;
        uint64 startBlock;
        uint64 endBlock;
        uint32 numSamples;
        uint64 updatedAt;
    }

    /// @notice Current round ID (incremented on each update).
    uint80 public currentRoundId;

    /// @notice Mapping of round ID to round data.
    mapping(uint80 => Round) internal rounds;

    /// @notice Emitted when the price feed is updated.
    event PriceUpdated(uint80 indexed roundId, int256 answer, uint64 startBlock, uint64 endBlock, uint32 numSamples);

    /// @notice Journal structure matching the guest program output.
    struct OracleJournal {
        bytes steelCommitment;
        address pool;
        uint256 medianPrice;
        uint8 journalDecimals;
        uint32 numSamples;
        uint64 startBlock;
        uint64 endBlock;
    }

    error InvalidPool(address expected, address received);
    error InvalidDecimals(uint8 expected, uint8 received);
    error RoundNotFound(uint80 roundId);
    error NoRoundData();

    constructor(
        IRiscZeroVerifier _verifier,
        bytes32 _imageId,
        address _pool,
        uint8 _decimals,
        string memory _description
    ) {
        VERIFIER = _verifier;
        IMAGE_ID = _imageId;
        POOL = _pool;
        PRICE_DECIMALS = _decimals;
        feedDescription = _description;
    }

    /// @notice Submit a ZK proof to update the price feed.
    /// @param journalData ABI-encoded OracleJournal from the guest program.
    /// @param seal The RISC Zero proof seal.
    function updatePrice(bytes calldata journalData, bytes calldata seal) external {
        // Decode the journal.
        OracleJournal memory journal = abi.decode(journalData, (OracleJournal));

        // Validate the journal matches this oracle's configuration.
        if (journal.pool != POOL) revert InvalidPool(POOL, journal.pool);
        if (journal.journalDecimals != PRICE_DECIMALS) revert InvalidDecimals(PRICE_DECIMALS, journal.journalDecimals);

        // Validate the Steel commitment (proves the data came from actual on-chain state).
        Steel.Commitment memory commitment = abi.decode(journal.steelCommitment, (Steel.Commitment));
        require(Steel.validateCommitment(commitment), "Invalid Steel commitment");

        // Verify the ZK proof.
        bytes32 journalHash = sha256(journalData);
        VERIFIER.verify(seal, IMAGE_ID, journalHash);

        // Store the new round.
        currentRoundId++;
        rounds[currentRoundId] = Round({
            answer: int256(journal.medianPrice),
            startBlock: journal.startBlock,
            endBlock: journal.endBlock,
            numSamples: journal.numSamples,
            updatedAt: uint64(block.timestamp)
        });

        emit PriceUpdated(currentRoundId, int256(journal.medianPrice), journal.startBlock, journal.endBlock, journal.numSamples);
    }

    // ─── AggregatorV3Interface ───────────────────────────────────────────

    /// @inheritdoc IAggregatorV3Interface
    function decimals() external view override returns (uint8) {
        return PRICE_DECIMALS;
    }

    /// @inheritdoc IAggregatorV3Interface
    function description() external view override returns (string memory) {
        return feedDescription;
    }

    /// @inheritdoc IAggregatorV3Interface
    function version() external pure override returns (uint256) {
        return 1;
    }

    /// @inheritdoc IAggregatorV3Interface
    function getRoundData(uint80 _roundId)
        external
        view
        override
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        Round storage round = rounds[_roundId];
        if (round.updatedAt == 0) revert RoundNotFound(_roundId);

        return (
            _roundId,
            round.answer,
            uint256(round.startBlock), // startedAt = start block number
            uint256(round.updatedAt),
            _roundId
        );
    }

    /// @inheritdoc IAggregatorV3Interface
    function latestRoundData()
        external
        view
        override
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        if (currentRoundId == 0) revert NoRoundData();
        Round storage round = rounds[currentRoundId];

        return (
            currentRoundId,
            round.answer,
            uint256(round.startBlock),
            uint256(round.updatedAt),
            currentRoundId
        );
    }

    // ─── View helpers ────────────────────────────────────────────────────

    /// @notice Returns the full round metadata including block range and sample count.
    function getRoundMetadata(uint80 _roundId)
        external
        view
        returns (uint64 startBlock, uint64 endBlock, uint32 numSamples, uint64 updatedAt)
    {
        Round storage round = rounds[_roundId];
        if (round.updatedAt == 0) revert RoundNotFound(_roundId);
        return (round.startBlock, round.endBlock, round.numSamples, round.updatedAt);
    }

    /// @notice Returns the image ID used for proof verification.
    function imageId() external view returns (bytes32) {
        return IMAGE_ID;
    }
}
