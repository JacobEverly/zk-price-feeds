// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import "forge-std/Test.sol";
import {ZkOracle, IAggregatorV3Interface} from "../src/ZkOracle.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";

/// @notice Mock verifier that accepts all proofs (for testing only).
contract MockVerifier is IRiscZeroVerifier {
    function verify(bytes calldata, bytes32, bytes32) external pure {}
    function verifyIntegrity(Receipt calldata) external pure {}
}

contract ZkOracleTest is Test {
    ZkOracle public oracle;
    MockVerifier public verifier;

    address constant POOL = address(0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc); // USDC/ETH
    bytes32 constant IMAGE_ID = bytes32(uint256(1)); // placeholder
    uint8 constant DECIMALS = 18;

    function setUp() public {
        verifier = new MockVerifier();
        oracle = new ZkOracle(
            IRiscZeroVerifier(address(verifier)),
            IMAGE_ID,
            POOL,
            DECIMALS,
            "ETH / USDC"
        );
    }

    function test_decimals() public view {
        assertEq(oracle.decimals(), DECIMALS);
    }

    function test_description() public view {
        assertEq(oracle.description(), "ETH / USDC");
    }

    function test_version() public view {
        assertEq(oracle.version(), 1);
    }

    function test_noRoundData() public {
        vm.expectRevert(ZkOracle.NoRoundData.selector);
        oracle.latestRoundData();
    }

    function test_roundNotFound() public {
        vm.expectRevert(abi.encodeWithSelector(ZkOracle.RoundNotFound.selector, uint80(99)));
        oracle.getRoundData(99);
    }

    function test_imageId() public view {
        assertEq(oracle.imageId(), IMAGE_ID);
    }

    function test_pool() public view {
        assertEq(oracle.POOL(), POOL);
    }
}
