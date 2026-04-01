// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Token} from "../src/Token.sol";

contract Token_Test is Test {
    Token public token;

    address payable developer = payable(makeAddr("developer"));
    address payable user = payable(makeAddr("user"));
    address payable canister = payable(makeAddr("canister"));

    function setUp() public {
        vm.broadcast(developer);
        token = new Token("ICP", 8, 10);
        vm.broadcast(developer);
        token.updateOwner(canister);
    }

    function test_mint() public {
        vm.broadcast(canister);
        token.mint(user, 42);
        assertEq(token.balanceOf(user), 42);
    }

    function test_user_cannot_mint() public {
        vm.broadcast(user);
        vm.expectRevert();
        token.mint(user, 42);
        assertEq(token.balanceOf(user), 0);
    }

    function test_burn1() public {
        vm.broadcast(canister);
        token.mint(user, 42);
        vm.broadcast(user);
        vm.expectEmit();
        emit Token.Burn1(user, 40, bytes32(uint256(123)));
        token.burn1(40, bytes32(uint256(123)));
        assertEq(token.balanceOf(user), 2);
    }

    function test_burn2() public {
        vm.broadcast(canister);
        token.mint(user, 42);
        vm.broadcast(user);
        vm.expectEmit();
        emit Token.Burn2(user, 40, bytes32(uint256(123)), bytes32(uint256(456)));
        token.burn2(40, bytes32(uint256(123)), bytes32(uint256(456)));
        assertEq(token.balanceOf(user), 2);
    }

    function test_burn3() public {
        vm.broadcast(canister);
        token.mint(user, 42);
        vm.broadcast(user);
        vm.expectEmit();
        emit Token.Burn3(user, 40, bytes32(uint256(123)), bytes32(uint256(456)), bytes32(uint256(789)));
        token.burn3(40, bytes32(uint256(123)), bytes32(uint256(456)), bytes32(uint256(789)));
        assertEq(token.balanceOf(user), 2);
    }

    function test_burn4() public {
        vm.broadcast(canister);
        token.mint(user, 42);
        vm.broadcast(user);
        vm.expectEmit();
        emit Token.Burn4(
            user, 40, bytes32(uint256(123)), bytes32(uint256(456)), bytes32(uint256(789)), bytes32(uint256(12))
        );
        token.burn4(40, bytes32(uint256(123)), bytes32(uint256(456)), bytes32(uint256(789)), bytes32(uint256(12)));
        assertEq(token.balanceOf(user), 2);
    }

    function testOnlyOwnerCanBatchMint() public {
        address payable[] memory recipients = new address payable[](1);
        uint256[] memory amounts = new uint256[](1);
        uint256[] memory ethAmounts = new uint256[](0);
        recipients[0] = payable(user);
        amounts[0] = 100;

        vm.broadcast(user);
        vm.expectRevert();
        token.batchMint(recipients, amounts, ethAmounts);

        vm.broadcast(canister);
        token.batchMint(recipients, amounts, ethAmounts);
        assertEq(token.balanceOf(user), 100);
    }

    function testTransferBatch() public {
        address payable[] memory recipients = new address payable[](3);
        uint256[] memory amounts = new uint256[](3);
        uint256[] memory ethAmounts = new uint256[](0);
        recipients[0] = payable(makeAddr("user1"));
        amounts[0] = 100;
        recipients[1] = payable(makeAddr("user2"));
        amounts[1] = 100;
        recipients[2] = payable(makeAddr("user3"));
        amounts[2] = 100;

        vm.broadcast(canister);
        token.batchMint(recipients, amounts, ethAmounts);

        assertEq(token.balanceOf(makeAddr("user1")), 100);
        assertEq(token.balanceOf(makeAddr("user2")), 100);
        assertEq(token.balanceOf(makeAddr("user3")), 100);
        assertEq(token.balanceOf(canister), 0);
    }

    function testTransferBatchWithEth() public {
        vm.deal(canister, 5 ether);
        assertEq(canister.balance, 5 ether);

        address payable[] memory recipients = new address payable[](3);
        uint256[] memory amounts = new uint256[](3);
        uint256[] memory ethAmounts = new uint256[](3);
        recipients[0] = payable(makeAddr("user1"));
        amounts[0] = 100;
        ethAmounts[0] = 0;
        recipients[1] = payable(makeAddr("user2"));
        amounts[1] = 100;
        ethAmounts[1] = 0.5 ether;
        recipients[2] = payable(makeAddr("user3"));
        amounts[2] = 100;
        ethAmounts[2] = 0.3 ether;

        vm.broadcast(canister);
        token.batchMint{value: 1 ether}(recipients, amounts, ethAmounts);

        assertEq(token.balanceOf(makeAddr("user1")), 100);
        assertEq(token.balanceOf(makeAddr("user2")), 100);
        assertEq(token.balanceOf(makeAddr("user3")), 100);
        assertEq(token.balanceOf(canister), 0);
        assertEq(recipients[0].balance, 0);
        assertEq(recipients[1].balance, 0.5 ether);
        assertEq(recipients[2].balance, 0.3 ether);
        assertEq(canister.balance, 4 ether);
        assertEq(address(token).balance, 0.2 ether);

        // Should not allow to withdraw more than the contract balance.
        vm.broadcast(canister);
        vm.expectRevert();
        token.withdrawEth(0.3 ether, canister);

        // Only canister should be able to withdraw
        vm.broadcast(user);
        vm.expectRevert();
        token.withdrawEth(0.2 ether, canister);

        vm.broadcast(canister);
        token.withdrawEth(0.2 ether, canister);
        assertEq(address(token).balance, 0 ether);
        assertEq(canister.balance, 4.2 ether);
    }
}
